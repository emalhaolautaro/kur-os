# Testing

> **Archivos:** `tests/`, `src/lib.rs` (sección de testing)
> **Propósito:** Framework de pruebas custom para un entorno `no_std` que corre sobre QEMU.

---

## ¿Por qué un framework custom?

El test runner estándar de Rust depende de `std`. En un kernel `no_std`, se usa `custom_test_frameworks`:

```rust
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]
```

---

## Trait `Testable`

```rust
pub trait Testable {
    fn run(&self);
}
```

Implementado para toda `T: Fn()`. Imprime el nombre del test antes de ejecutarlo y `[ok]` después:

```
kur_os::vga_buffer::test_println_simple...    [ok]
```

Usa `core::any::type_name::<T>()` para obtener el nombre completo del test.

---

## `test_runner`

```rust
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Ejecutando {} pruebas", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}
```

Ejecuta todos los tests secuencialmente y sale de QEMU con código de éxito.

---

## Salida de QEMU (`exit_qemu`)

```rust
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed  = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    let mut port = Port::new(0xf4);
    port.write(exit_code as u32);
}
```

El dispositivo `isa-debug-exit` (configurado en `Cargo.toml`) mapea el puerto `0xf4`. El código de salida real es `(valor << 1) | 1`:
- Success: `(0x10 << 1) | 1 = 33` → configurado como `test-success-exit-code`
- Failed: `(0x11 << 1) | 1 = 35`

---

## Tests de integración

### `tests/basic_boot.rs`

| Test | Qué verifica |
|------|-------------|
| `test_println` | Que `println!` funciona después del boot |

> **Nota:** Usa `_start` como entry point (sin bootloader entry_point macro).

### `tests/should_panic.rs`

Test que **debe** hacer panic. Usa su propio `#[panic_handler]` que reporta éxito:
- Si `should_fail()` hace panic → `[ok]` + exit success
- Si no hace panic → `[la prueba no falló]` + exit failed

Configurado en `Cargo.toml` con `harness = false` (no usa el test runner).

### `tests/stack_overflow.rs`

Verifica que el double fault handler funciona ante stack overflow:
1. Carga una IDT custom con un handler que reporta `[ok]`
2. Ejecuta recursión infinita
3. El CPU dispara double fault → handler → exit success

También `harness = false`. Carga su propia IDT para no depender de la del kernel.

### `tests/memory.rs`

| Test | Qué verifica |
|------|-------------|
| `test_vga_buffer_is_mapped` | Que `0xb8000` es accesible (lectura/escritura volátil) |
| `test_kernel_code_is_accessible` | Que los punteros a funciones del kernel no son null |
| `test_stack_is_accessible` | Que se puede escribir y leer del stack correctamente |

### `tests/heap_allocation.rs`

| Test | Qué verifica |
|------|-------------|
| `simple_allocation` | Dos `Box::new()` retienen sus valores |
| `large_vec` | Un `Vec` de 1000 elementos tiene la suma correcta |
| `many_boxes` | `HEAP_SIZE` asignaciones+liberaciones sucesivas (verifica reuso) |
| `fragmentation` | Asignar 1000 bloques de 16B, liberar 50%, luego asignar 500 bloques de 32B (verifica que el allocator maneja fragmentación) |

---

## Entry point para tests en `lib.rs`

Cuando se compila con `cfg(test)`, `lib.rs` define su propio entry point:

```rust
#[cfg(test)]
entry_point!(test_kernel_main);

#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    init();
    test_main();
    hlt_loop();
}
```

Esto permite que `cargo test --lib` ejecute los tests unitarios definidos dentro de los módulos (como los de `vga_buffer`).
