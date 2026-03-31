//! quench Token — ERC-20-совместимый смарт-контракт.
//!
//! ## Компиляция в WASM
//!
//! ```bash
//! rustup target add wasm32-unknown-unknown
//! cargo build --target wasm32-unknown-unknown --release
//! # Оптимизируем размер:
//! wasm-opt -Oz target/wasm32-unknown-unknown/release/token.wasm -o token_opt.wasm
//! ```
//!
//! ## Интерфейс
//!
//! Экспортируемые функции (вызываются нодой):
//! - `init(owner_ptr, owner_len, total_supply)` — деплой
//! - `transfer(to_ptr, to_len, amount)` → 1 (успех) | 0 (ошибка)
//! - `balance_of(addr_ptr, addr_len)` → u64
//! - `approve(spender_ptr, spender_len, amount)` → 1 | 0
//! - `transfer_from(from_ptr, from_len, to_ptr, to_len, amount)` → 1 | 0
//! - `allowance(owner_ptr, owner_len, spender_ptr, spender_len)` → u64

#![no_std]
#![no_main]

// В WASM нет стандартного аллокатора — подключаем wee_alloc
extern crate alloc;
use alloc::vec::Vec;
use alloc::format;

// ─── Host imports ────────────────────────────────────────────────────────────
// Функции, предоставляемые нодой (rc-vm/host.rs)

extern "C" {
    fn storage_get(key_ptr: i32, key_len: i32) -> i32;
    fn storage_set(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32);
    fn get_caller(buf_ptr: i32);
    fn emit_event(name_ptr: i32, name_len: i32, data_ptr: i32, data_len: i32);
}

// ─── Storage keys ─────────────────────────────────────────────────────────────

fn balance_key(address: &[u8]) -> Vec<u8> {
    let mut key = b"balance:".to_vec();
    key.extend_from_slice(address);
    key
}

fn allowance_key(owner: &[u8], spender: &[u8]) -> Vec<u8> {
    let mut key = b"allowance:".to_vec();
    key.extend_from_slice(owner);
    key.push(b':');
    key.extend_from_slice(spender);
    key
}

const TOTAL_SUPPLY_KEY: &[u8] = b"total_supply";
const NAME_KEY:         &[u8] = b"name";
const SYMBOL_KEY:       &[u8] = b"symbol";
const DECIMALS_KEY:     &[u8] = b"decimals";

// ─── Storage helpers ─────────────────────────────────────────────────────────

static mut SCRATCH_BUF: [u8; 1024] = [0u8; 1024];

fn store_u64(key: &[u8], value: u64) {
    let bytes = value.to_le_bytes();
    unsafe {
        storage_set(
            key.as_ptr() as i32, key.len() as i32,
            bytes.as_ptr() as i32, 8,
        );
    }
}

fn load_u64(key: &[u8]) -> u64 {
    let len = unsafe {
        storage_get(key.as_ptr() as i32, key.len() as i32)
    };
    if len != 8 { return 0; }

    unsafe {
        let mut bytes = [0u8; 8];
        // Читаем через scratch buffer (упрощение — в реальности через result_get)
        u64::from_le_bytes(bytes)
    }
}

fn emit(name: &[u8], data: &[u8]) {
    unsafe {
        emit_event(
            name.as_ptr() as i32, name.len() as i32,
            data.as_ptr() as i32, data.len() as i32,
        );
    }
}

fn caller() -> [u8; 20] {
    let mut buf = [0u8; 20];
    unsafe { get_caller(buf.as_mut_ptr() as i32); }
    buf
}

// ─── Exported functions ───────────────────────────────────────────────────────

/// Инициализация контракта (вызывается при деплое)
#[no_mangle]
pub extern "C" fn init(
    owner_ptr: i32,
    owner_len: i32,
    total_supply: i64,
    name_ptr: i32,
    name_len: i32,
    symbol_ptr: i32,
    symbol_len: i32,
) {
    let total = total_supply as u64;
    let owner = unsafe {
        core::slice::from_raw_parts(owner_ptr as *const u8, owner_len as usize)
    };
    let name = unsafe {
        core::slice::from_raw_parts(name_ptr as *const u8, name_len as usize)
    };
    let symbol = unsafe {
        core::slice::from_raw_parts(symbol_ptr as *const u8, symbol_len as usize)
    };

    // Записываем метаданные
    store_u64(TOTAL_SUPPLY_KEY, total);
    store_u64(DECIMALS_KEY, 8);
    unsafe {
        storage_set(NAME_KEY.as_ptr() as i32, NAME_KEY.len() as i32,
                    name.as_ptr() as i32, name.len() as i32);
        storage_set(SYMBOL_KEY.as_ptr() as i32, SYMBOL_KEY.len() as i32,
                    symbol.as_ptr() as i32, symbol.len() as i32);
    }

    // Все токены → owner
    let key = balance_key(owner);
    store_u64(&key, total);

    emit(b"Transfer", b"{\"from\":\"0x0\",\"amount\":\"all\"}");
}

/// Перевести токены получателю
#[no_mangle]
pub extern "C" fn transfer(to_ptr: i32, to_len: i32, amount: i64) -> i32 {
    let amount = amount as u64;
    let to     = unsafe {
        core::slice::from_raw_parts(to_ptr as *const u8, to_len as usize)
    };
    let from = caller();

    let from_key = balance_key(&from);
    let from_bal = load_u64(&from_key);

    if from_bal < amount { return 0; } // недостаточно средств

    let to_key  = balance_key(to);
    let to_bal  = load_u64(&to_key);

    store_u64(&from_key, from_bal - amount);
    store_u64(&to_key,   to_bal + amount);

    emit(b"Transfer", b"{}");
    1 // успех
}

/// Узнать баланс адреса
#[no_mangle]
pub extern "C" fn balance_of(addr_ptr: i32, addr_len: i32) -> i64 {
    let addr = unsafe {
        core::slice::from_raw_parts(addr_ptr as *const u8, addr_len as usize)
    };
    let key = balance_key(addr);
    load_u64(&key) as i64
}

/// Разрешить spender тратить amount от имени caller
#[no_mangle]
pub extern "C" fn approve(spender_ptr: i32, spender_len: i32, amount: i64) -> i32 {
    let spender = unsafe {
        core::slice::from_raw_parts(spender_ptr as *const u8, spender_len as usize)
    };
    let owner = caller();
    let key   = allowance_key(&owner, spender);
    store_u64(&key, amount as u64);

    emit(b"Approval", b"{}");
    1
}

/// Перевод от имени owner (если есть allowance)
#[no_mangle]
pub extern "C" fn transfer_from(
    from_ptr: i32, from_len: i32,
    to_ptr:   i32, to_len:   i32,
    amount:   i64,
) -> i32 {
    let amount  = amount as u64;
    let from    = unsafe { core::slice::from_raw_parts(from_ptr as *const u8, from_len as usize) };
    let to      = unsafe { core::slice::from_raw_parts(to_ptr   as *const u8, to_len   as usize) };
    let spender = caller();

    let allowance_key = allowance_key(from, &spender);
    let allowance     = load_u64(&allowance_key);
    if allowance < amount { return 0; }

    let from_key = balance_key(from);
    let from_bal = load_u64(&from_key);
    if from_bal < amount { return 0; }

    let to_key  = balance_key(to);
    let to_bal  = load_u64(&to_key);

    store_u64(&from_key,     from_bal - amount);
    store_u64(&to_key,       to_bal   + amount);
    store_u64(&allowance_key, allowance - amount);

    emit(b"Transfer", b"{}");
    1
}

/// Оставшийся allowance
#[no_mangle]
pub extern "C" fn allowance(
    owner_ptr:   i32, owner_len:   i32,
    spender_ptr: i32, spender_len: i32,
) -> i64 {
    let owner   = unsafe { core::slice::from_raw_parts(owner_ptr   as *const u8, owner_len   as usize) };
    let spender = unsafe { core::slice::from_raw_parts(spender_ptr as *const u8, spender_len as usize) };
    let key     = allowance_key(owner, spender);
    load_u64(&key) as i64
}

/// Функция выделения памяти (нода использует для передачи аргументов)
#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let layout = alloc::alloc::Layout::array::<u8>(size as usize).unwrap();
    unsafe { alloc::alloc::alloc(layout) as i32 }
}

// Паника в WASM — тихая, просто зависаем
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
