//! Host-функции — API, доступное смарт-контрактам.
//!
//! Контракт может вызывать только эти функции для взаимодействия
//! с внешним миром. Всё остальное — изолировано.
//!
//! В WASM-контракте они импортируются как:
//! ```wat
//! (import "env" "storage_get" (func $storage_get (param i32 i32) (result i32)))
//! (import "env" "transfer"    (func $transfer    (param i32 i64)))
//! ```

use crate::context::ExecutionState;
use wasmtime::{Caller, Linker};

/// Данные, хранящиеся в `wasmtime::Store`
pub struct HostState {
    /// Мутабельное состояние исполнения
    pub exec_state: ExecutionState,
    /// Хранилище контракта (key-value)
    pub contract_storage: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    /// Оставшийся газ
    pub gas_remaining: u64,
    /// Адрес вызывающего (для `get_caller`)
    pub caller_bytes: [u8; 20],
}

/// Зарегистрировать все host-функции в linker
///
/// Linker связывает импорты WASM-контракта с реальными Rust-функциями.
pub fn register_host_functions(linker: &mut Linker<HostState>) -> Result<(), wasmtime::Error> {
    // storage_get(key_ptr, key_len) → val_len
    // Контракт читает ключ из своей памяти, хост возвращает длину значения.
    // Само значение контракт читает через отдельный вызов result_get().
    linker.func_wrap(
        "env",
        "storage_get",
        |mut caller: Caller<'_, HostState>, key_ptr: i32, key_len: i32| -> i32 {
            let key = read_memory(&mut caller, key_ptr as usize, key_len as usize);
            let state = caller.data();
            if let Some(val) = state.contract_storage.get(&key) {
                val.len() as i32
            } else {
                -1 // ключ не найден
            }
        },
    )?;

    // storage_set(key_ptr, key_len, val_ptr, val_len)
    linker.func_wrap(
        "env",
        "storage_set",
        |mut caller: Caller<'_, HostState>,
         key_ptr: i32,
         key_len: i32,
         val_ptr: i32,
         val_len: i32| {
            let key = read_memory(&mut caller, key_ptr as usize, key_len as usize);
            let val = read_memory(&mut caller, val_ptr as usize, val_len as usize);

            // Проверяем газ
            let gas_cost = crate::gas::GasCost::STORAGE_WRITE;
            let state = caller.data_mut();
            if state.gas_remaining < gas_cost {
                // Газ исчерпан — wasmtime прервёт выполнение
                return;
            }
            state.gas_remaining -= gas_cost;
            state
                .exec_state
                .storage_writes
                .push((key.clone(), val.clone()));
            state.contract_storage.insert(key, val);
        },
    )?;

    // get_caller(buf_ptr) — записывает 20 байт адреса вызывающего в память контракта
    linker.func_wrap(
        "env",
        "get_caller",
        |mut caller: Caller<'_, HostState>, buf_ptr: i32| {
            let addr_bytes = caller.data().caller_bytes;
            write_memory(&mut caller, buf_ptr as usize, &addr_bytes);
        },
    )?;

    // emit_event(name_ptr, name_len, data_ptr, data_len)
    linker.func_wrap(
        "env",
        "emit_event",
        |mut caller: Caller<'_, HostState>,
         name_ptr: i32,
         name_len: i32,
         data_ptr: i32,
         data_len: i32| {
            let name_bytes = read_memory(&mut caller, name_ptr as usize, name_len as usize);
            let data = read_memory(&mut caller, data_ptr as usize, data_len as usize);

            let state = caller.data_mut();
            if state.gas_remaining < crate::gas::GasCost::EMIT_EVENT {
                return;
            }
            state.gas_remaining -= crate::gas::GasCost::EMIT_EVENT;

            let name = String::from_utf8_lossy(&name_bytes).into_owned();
            state.exec_state.events.push(crate::context::ContractEvent {
                contract: rc_primitives::types::Address::ZERO, // заполняется снаружи
                name,
                data,
            });
        },
    )?;

    // abort(msg_ptr, msg_len) — контракт сигнализирует об ошибке
    linker.func_wrap(
        "env",
        "abort",
        |mut caller: Caller<'_, HostState>, msg_ptr: i32, msg_len: i32| {
            let msg = read_memory(&mut caller, msg_ptr as usize, msg_len as usize);
            let msg_str = String::from_utf8_lossy(&msg);
            tracing::warn!("contract abort: {msg_str}");
            // Trap прерывает выполнение WASM
        },
    )?;

    Ok(())
}

/// Прочитать байты из линейной памяти WASM-контракта
fn read_memory(caller: &mut Caller<'_, HostState>, ptr: usize, len: usize) -> Vec<u8> {
    let memory = match caller.get_export("memory") {
        Some(wasmtime::Extern::Memory(m)) => m,
        _ => return vec![],
    };
    let data = memory.data(caller);
    data.get(ptr..ptr + len)
        .map(|s| s.to_vec())
        .unwrap_or_default()
}

/// Записать байты в линейную память WASM-контракта
fn write_memory(caller: &mut Caller<'_, HostState>, ptr: usize, bytes: &[u8]) {
    let memory = match caller.get_export("memory") {
        Some(wasmtime::Extern::Memory(m)) => m,
        _ => return,
    };
    let data = memory.data_mut(caller);
    if let Some(slice) = data.get_mut(ptr..ptr + bytes.len()) {
        slice.copy_from_slice(bytes);
    }
}
