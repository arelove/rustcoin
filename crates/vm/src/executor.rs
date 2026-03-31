//! Главный исполнитель WASM смарт-контрактов.

use crate::{
    context::{ExecutionContext, ExecutionState},
    error::VmError,
    host::{register_host_functions, HostState},
};
use wasmtime::{Config, Engine, Linker, Module, Store};

/// Результат исполнения смарт-контракта
#[derive(Debug)]
pub struct ExecutionResult {
    /// Успех или описание ошибки
    pub success: bool,
    /// Возвращённые данные (ABI-encoded)
    pub return_data: Vec<u8>,
    /// Использованный газ
    pub gas_used: u64,
    /// Изменения state (storage writes, transfers, events)
    pub state: ExecutionState,
    /// Сообщение об ошибке (если !success)
    pub error_message: Option<String>,
}

/// Исполнитель контрактов
///
/// `Engine` и скомпилированные `Module` можно кэшировать и переиспользовать.
/// Один `Executor` может исполнять тысячи вызовов.
pub struct Executor {
    engine: Engine,
}

impl Executor {
    /// Создать новый исполнитель
    pub fn new() -> Result<Self, VmError> {
        let mut config = Config::new();
        // Fuel-based execution: каждая WASM инструкция "сжигает" топливо
        config.consume_fuel(true);
        // Эпоха прерывания (для timeout)
        config.epoch_interruption(true);
        // Оптимизируем для скорости
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);

        let engine = Engine::new(&config).map_err(|e| VmError::Init(e.to_string()))?;
        Ok(Self { engine })
    }

    /// Задеплоить новый контракт
    ///
    /// Компилирует WASM байткод и вызывает `init(args)`.
    /// Возвращает начальный state контракта.
    pub fn deploy(
        &self,
        bytecode: &[u8],
        ctx: ExecutionContext,
        init_storage: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<ExecutionResult, VmError> {
        // Компилируем WASM (нативный код, быстро исполняется)
        let module =
            Module::new(&self.engine, bytecode).map_err(|e| VmError::Compile(e.to_string()))?;

        let gas_limit = ctx.gas_limit;
        let caller_bytes = *ctx.caller.as_bytes();

        let mut store = self.make_store(
            HostState {
                exec_state: ExecutionState::default(),
                contract_storage: init_storage,
                gas_remaining: gas_limit,
                caller_bytes,
            },
            gas_limit,
        );

        let mut linker = Linker::new(&self.engine);
        register_host_functions(&mut linker).map_err(|e| VmError::Init(e.to_string()))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| VmError::Instantiate(e.to_string()))?;

        // Вызываем init если она есть
        if let Ok(init_fn) = instance.get_typed_func::<(), ()>(&mut store, "init") {
            init_fn
                .call(&mut store, ())
                .map_err(|e| VmError::Execution(e.to_string()))?;
        }

        let gas_used = gas_limit - store.data().gas_remaining;
        let state = std::mem::take(&mut store.data_mut().exec_state);

        Ok(ExecutionResult {
            success: true,
            return_data: vec![],
            gas_used,
            state,
            error_message: None,
        })
    }

    /// Вызвать функцию контракта
    pub fn call(
        &self,
        bytecode: &[u8],
        ctx: ExecutionContext,
        storage: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<ExecutionResult, VmError> {
        let module =
            Module::new(&self.engine, bytecode).map_err(|e| VmError::Compile(e.to_string()))?;

        let gas_limit = ctx.gas_limit;
        let caller_bytes = *ctx.caller.as_bytes();
        let method = ctx.method.clone();
        let args = ctx.args.clone();

        let mut store = self.make_store(
            HostState {
                exec_state: ExecutionState::default(),
                contract_storage: storage,
                gas_remaining: gas_limit,
                caller_bytes,
            },
            gas_limit,
        );

        let mut linker = Linker::new(&self.engine);
        register_host_functions(&mut linker).map_err(|e| VmError::Init(e.to_string()))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| VmError::Instantiate(e.to_string()))?;

        // Большинство контрактов экспортируют функцию `call(method_ptr, method_len, args_ptr, args_len)`
        // Для простоты мы ищем функцию по имени метода напрямую
        let result = match instance.get_typed_func::<(i32, i32), i32>(&mut store, &method) {
            Ok(func) => {
                // Записываем args в memory контракта
                let (args_ptr, args_len) =
                    Self::write_args_to_memory(&instance, &mut store, &args).unwrap_or((0, 0));

                func.call(&mut store, (args_ptr, args_len))
                    .map(|_| vec![])
                    .map_err(|e| e.to_string())
            }
            Err(_) => Err(format!("method '{}' not found in contract", method)),
        };

        let gas_used = gas_limit.saturating_sub(store.data().gas_remaining);
        let state = std::mem::take(&mut store.data_mut().exec_state);

        match result {
            Ok(return_data) => Ok(ExecutionResult {
                success: true,
                return_data,
                gas_used,
                state,
                error_message: None,
            }),
            Err(e) => Ok(ExecutionResult {
                success: false,
                return_data: vec![],
                gas_used,
                state,
                error_message: Some(e),
            }),
        }
    }

    fn make_store(&self, host: HostState, gas_limit: u64) -> Store<HostState> {
        let mut store = Store::new(&self.engine, host);
        // Устанавливаем fuel = gas_limit (1 fuel = 1 gas)
        store.set_fuel(gas_limit).ok();
        store
    }

    fn write_args_to_memory(
        instance: &wasmtime::Instance,
        store: &mut Store<HostState>,
        args: &[u8],
    ) -> Option<(i32, i32)> {
        if args.is_empty() {
            return Some((0, 0));
        }

        let alloc = instance
            .get_typed_func::<i32, i32>(&mut *store, "alloc") // reborrow
            .ok()?;
        let ptr = alloc.call(&mut *store, args.len() as i32).ok()?; // reborrow

        let memory = instance.get_memory(&mut *store, "memory")?; // reborrow
        let data = memory.data_mut(store); // last use, can consume
        let slice = data.get_mut(ptr as usize..ptr as usize + args.len())?;
        slice.copy_from_slice(args);

        Some((ptr, args.len() as i32))
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new().expect("wasmtime executor init")
    }
}
