# VGA Buffer

> **Archivo:** `src/vga_buffer.rs`
> **Propósito:** Proveer salida de texto por pantalla escribiendo directamente al buffer VGA mapeado en `0xb8000`.

---

## Concepto

En modo texto, la tarjeta de video expone un buffer en la dirección física `0xb8000`. Este buffer es una grilla de 25 filas × 80 columnas. Cada celda ocupa 2 bytes:

```
Byte 0: código ASCII del carácter
Byte 1: atributo de color (4 bits foreground + 4 bits background)
```

---

## Estructuras de datos

### `Color` (enum)

Enum con los 16 colores del modo texto VGA (0–15). Usa `#[repr(u8)]` para garantizar que cada variante ocupe exactamente 1 byte.

### `ColorCode`

Wrapper transparente (`#[repr(transparent)]`) sobre un `u8`. Empaqueta foreground y background en un solo byte:

```rust
ColorCode((background as u8) << 4 | (foreground as u8))
```

### `ScreenChar`

Par `(ascii_character, color_code)` con `#[repr(C)]` para garantizar el layout en memoria que el hardware espera.

### `Buffer`

Matriz `[[Volatile<ScreenChar>; 80]; 25]` con `#[repr(transparent)]`. Se usa `Volatile` del crate `volatile` para evitar que el compilador optimice las escrituras al buffer (dead store elimination).

---

## `Writer`

Estructura principal que mantiene el estado de escritura:

- `column_position`: columna actual del cursor
- `color_code`: colores activos (actualmente amarillo sobre negro)
- `buffer`: referencia `&'static mut` al buffer VGA

### Métodos

| Método | Descripción |
|--------|-------------|
| `write_byte(byte)` | Escribe un byte. Si es `\n`, hace new line. Si la columna llega a 80, también hace new line |
| `new_line()` | Desplaza todas las filas una posición arriba (scroll) y limpia la última fila |
| `clear_row(row)` | Llena una fila con espacios en blanco |
| `write_string(s)` | Escribe un string byte a byte. Caracteres fuera del rango ASCII imprimible (`0x20..=0x7e`) se reemplazan por `■` (`0xfe`) |

### Implementación de `fmt::Write`

`Writer` implementa `core::fmt::Write`, lo que permite usar `write!` y `write_fmt` con él. Esto es la base de los macros `print!` y `println!`.

---

## Instancia global `WRITER`

```rust
lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}
```

- **`lazy_static!`** porque necesitamos inicializar la referencia al buffer en runtime (raw pointer cast)
- **`spin::Mutex`** para sincronización sin bloqueo (no hay sistema operativo debajo)
- El color por defecto es **amarillo sobre negro**

---

## Macros `print!` y `println!`

```rust
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}
```

La función `_print` adquiere el lock del `WRITER` dentro de `interrupts::without_interrupts` para evitar deadlocks si una interrupción intenta imprimir mientras el lock está tomado.

---

## Tests en el módulo

| Test | Qué verifica |
|------|-------------- |
| `test_println_simple` | Que un `println!` simple no produce panic |
| `test_println_many` | Que 200 `println!` consecutivos no producen panic (prueba de scroll) |
| `test_println_output` | Que el texto escrito realmente aparece en la posición correcta del buffer VGA |

> El test `test_println_output` usa `without_interrupts` para evitar que el timer interrupt inserte un `.` entre la escritura y la lectura del buffer.
