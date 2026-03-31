use rc_consensus::{
    difficulty::{compute_target, ADJUSTMENT_INTERVAL, TARGET_BLOCK_TIME_MS},
    fork_choice::ForkChoice,
    miner::{Miner, BLOCK_REWARD_INITIAL, HALVING_INTERVAL},
};
use rc_primitives::{
    block::Block,
    hash::Hash,
    types::{Amount, BlockHeight, Timestamp},
};
use tokio_util::sync::CancellationToken;

// ─── Mining ───────────────────────────────────────────────────────────────────

#[test]
fn test_block_reward_halving() {
    // Начальная награда
    assert_eq!(Miner::block_reward(BlockHeight(0)), BLOCK_REWARD_INITIAL);

    // После первого халвинга
    let after_first = Miner::block_reward(BlockHeight(HALVING_INTERVAL));
    assert_eq!(after_first, Amount(BLOCK_REWARD_INITIAL.0 / 2));

    // После второго халвинга
    let after_second = Miner::block_reward(BlockHeight(HALVING_INTERVAL * 2));
    assert_eq!(after_second, Amount(BLOCK_REWARD_INITIAL.0 / 4));

    // После 64+ халвингов — 0
    let zero = Miner::block_reward(BlockHeight(HALVING_INTERVAL * 65));
    assert_eq!(zero, Amount(0));
}

#[tokio::test]
async fn test_mine_low_difficulty() {
    // Difficulty=1 означает 1 нулевой бит — очень быстро
    let cancel = CancellationToken::new();
    let addr = rc_primitives::types::Address::from_bytes([9u8; 20]);
    let miner = Miner::new(addr, cancel);

    let result = miner
        .mine_async(Hash::ZERO, BlockHeight(1), vec![], 1, 1)
        .await
        .expect("should mine quickly at difficulty=1");

    assert!(result.block.header.meets_difficulty());
    assert!(result.attempts > 0);
    assert_eq!(result.block.height().0, 1);
    // Первая транзакция должна быть Coinbase
    let first_tx = &result.block.transactions[0];
    assert!(matches!(
        first_tx.kind,
        rc_primitives::transaction::TxKind::Coinbase
    ));
}

#[tokio::test]
async fn test_mining_cancellation() {
    let cancel = CancellationToken::new();
    let addr = rc_primitives::types::Address::from_bytes([9u8; 20]);
    let miner = Miner::new(addr, cancel.clone());

    // Отменяем немедленно
    cancel.cancel();

    let result = miner
        .mine_async(Hash::ZERO, BlockHeight(1), vec![], 30, 1) // высокая сложность
        .await;

    assert!(
        matches!(
            result,
            Err(rc_consensus::error::ConsensusError::MiningCancelled)
        ),
        "cancelled mining should return MiningCancelled"
    );
}

// ─── Difficulty ───────────────────────────────────────────────────────────────

#[test]
fn test_difficulty_no_change_on_target() {
    let expected_ms = ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME_MS;
    let start = Timestamp(0);
    let end = Timestamp(expected_ms);
    let new_bits = compute_target(20, start, end);
    // Если время = ожидаемому → сложность не меняется
    assert_eq!(new_bits, 20);
}

#[test]
fn test_difficulty_increases_if_too_fast() {
    // Блоки шли в 2 раза быстрее → сложность должна вырасти
    let expected_ms = ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME_MS;
    let actual_ms = expected_ms / 2;
    let start = Timestamp(0);
    let end = Timestamp(actual_ms);
    let new_bits = compute_target(20, start, end);
    assert!(
        new_bits > 20,
        "faster blocks → higher difficulty, got {new_bits}"
    );
}

#[test]
fn test_difficulty_decreases_if_too_slow() {
    // Блоки шли в 2 раза медленнее → сложность должна упасть
    let expected_ms = ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME_MS;
    let actual_ms = expected_ms * 2;
    let start = Timestamp(0);
    let end = Timestamp(actual_ms);
    let new_bits = compute_target(20, start, end);
    assert!(
        new_bits < 20,
        "slower blocks → lower difficulty, got {new_bits}"
    );
}

#[test]
fn test_difficulty_clamped_to_min() {
    // Очень медленные блоки → не падает ниже MIN_DIFFICULTY
    let start = Timestamp(0);
    let end = Timestamp(u64::MAX / 2);
    let new_bits = compute_target(20, start, end);
    assert!(new_bits >= rc_consensus::difficulty::MIN_DIFFICULTY);
}

#[test]
fn test_difficulty_clamped_to_4x() {
    // Не может вырасти более чем в 4 раза за период
    let start = Timestamp(0);
    let end = Timestamp(1); // почти мгновенно
    let new_bits = compute_target(20, start, end);
    assert!(new_bits <= 20 * 4);
}

// ─── Fork Choice ─────────────────────────────────────────────────────────────

#[test]
fn test_fork_choice_selects_highest_work() {
    let mut fc = ForkChoice::new();

    let genesis = Block::genesis();
    let genesis_header = genesis.header.clone();
    fc.add_block(genesis_header);

    // Симулируем два конкурирующих блока на высоте 1
    let mut header_a = Block::genesis().header;
    header_a.height = rc_primitives::types::BlockHeight(1);
    header_a.bits = 20; // normal difficulty
    let _hash_a = header_a.compute_hash();
    fc.add_block(header_a);

    let mut header_b = Block::genesis().header;
    header_b.height = rc_primitives::types::BlockHeight(1);
    header_b.bits = 24; // higher difficulty → more work
    header_b.nonce = rc_primitives::types::Nonce(999); // разный hash
    let hash_b = header_b.compute_hash();
    let is_new_best = fc.add_block(header_b);

    // Блок B должен стать лучшим (у него больше work)
    assert!(
        is_new_best,
        "higher difficulty block should become best tip"
    );
    assert_eq!(fc.best_tip().unwrap(), hash_b);
}

#[test]
fn test_fork_choice_ancestry() {
    let mut fc = ForkChoice::new();

    let genesis = Block::genesis();
    let genesis_hash = genesis.hash();
    fc.add_block(genesis.header.clone());

    let mut h1 = genesis.header.clone();
    h1.height = rc_primitives::types::BlockHeight(1);
    h1.previous_hash = genesis_hash;
    fc.add_block(h1.clone());

    let h1_hash = h1.compute_hash();
    let ancestry = fc.ancestry(h1_hash);

    assert_eq!(ancestry.len(), 2, "genesis + block 1");
    assert_eq!(ancestry[0], genesis_hash);
    assert_eq!(ancestry[1], h1_hash);
}
