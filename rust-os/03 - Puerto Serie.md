# Puerto Serie (UART)

> **Archivo:** `src/serial.rs`
> **Propósito:** Comunicación con el host a través del puerto serie. Se usa principalmente para la salida de tests (donde el VGA no está visible).

---

## Diseño

El módulo es un wrapper delgado sobre el crate `uart_16550`, que maneja el hardware UART 16550.

### Puerto `SERIAL1`

```rust
lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}
```

- **`0x3F8`** es la dirección de I/O estándar de COM1 en x86
- Se inicializa con `lazy_static!` y se protege con `spin::Mutex`
- `serial_port.init()` configura los registros UART (baud rate, bits de datos, etc.)

### Función `_print`

Igual que en el VGA buffer, usa `interrupts::without_interrupts` para evitar deadlocks:

```rust
pub fn _print(args: core::fmt::Arguments) {
    interrupts::without_interrupts(|| {
        SERIAL1.lock().write_fmt(args).expect("...");
    });
}
```

---

## Macros

| Macro | Uso |
|-------|-----|
| `serial_print!` | Imprime al puerto serie sin newline |
| `serial_println!` | Imprime al puerto serie con newline |

Estos macros son fundamentales para el framework de testing: la salida de tests se redirige al puerto serie, que QEMU reenvía a `stdio` gracias a `-serial stdio`.

---

## Relación con QEMU

En la configuración de `Cargo.toml`:

```toml
run-args = ["-serial", "stdio"]
```

Esto hace que todo lo escrito al puerto serie aparezca en la terminal del host. Es la forma principal de ver resultados de tests sin tener que inspeccionar la pantalla VGA.
