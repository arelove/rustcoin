// crates/node/tests/apply_block_test.rs
//
// Интеграционный тест: проверяем что нода корректно инициализирует
// genesis и восстанавливает best_tip из БД.

use rc_primitives::block::Block;
use rc_storage::Database;

/// Открываем БД во временной директории средствами std (без tempfile в зависимостях)
fn make_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = Database::open(dir.path()).expect("open db");
    (db, dir) // держим dir живым — иначе директория удалится
}

#[test]
fn test_genesis_apply_and_best_tip() {
    let (db, _dir) = make_test_db();

    let genesis = Block::genesis();
    db.apply_block(&genesis).expect("apply genesis");

    // best_tip указывает на genesis
    let (hash, height) = db.get_best_tip().unwrap().unwrap();
    assert_eq!(hash, genesis.hash());
    assert_eq!(height.0, 0);
}

#[test]
fn test_genesis_idempotent() {
    let (db, _dir) = make_test_db();

    let genesis = Block::genesis();
    db.apply_block(&genesis).expect("first apply");
    db.apply_block(&genesis).expect("second apply — no-op");

    // best_tip всё ещё на genesis
    let (_, height) = db.get_best_tip().unwrap().unwrap();
    assert_eq!(height.0, 0);
}
