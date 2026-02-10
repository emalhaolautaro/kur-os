# GDT y TSS

> **Archivo:** `src/gdt.rs`
> **Propósito:** Configurar la Global Descriptor Table y el Task State Segment para segmentación y stacks de interrupción.

---

## ¿Qué es la GDT?

La **Global Descriptor Table** es una estructura del CPU x86 que define segmentos de memoria. En modo 64-bit (long mode), la segmentación clásica está mayormente deshabilitada, pero la GDT sigue siendo necesaria para:

1. **Definir el segmento de código del kernel** (selecciona modo 64-bit)
2. **Definir el segmento de datos del kernel** (necesario para que `SS` sea válido)
3. **Apuntar al TSS** (para stacks de interrupción)

---

## ¿Qué es el TSS?

El **Task State Segment** en x86_64 ya no se usa para cambio de tarea (eso se hace por software). Su rol principal es definir la **Interrupt Stack Table (IST)**: hasta 7 stacks alternativos que el CPU puede usar automáticamente al manejar ciertas excepciones.

### ¿Por qué necesitamos stacks alternativos?

Si ocurre un stack overflow, el stack pointer apunta a memoria inválida. Cuando el CPU intenta empujar el stack frame de la excepción, provoca un **double fault**, que a su vez intenta empujar otro frame en el mismo stack corrupto → **triple fault** → reinicio.

Con la IST, podemos decirle al CPU: "para double fault, usá este otro stack" y la cadena se corta.

---

## Diseño actual

### Stacks definidos en la IST

| Índice IST | Constante | Usado por | Tamaño |
|------------|-----------|-----------|--------|
| 0 | `DOUBLE_FAULT_IST_INDEX` | Double fault handler | 20 KB (4096 × 5) |
| 1 | `BREAKPOINT_IST_INDEX` | Breakpoint handler | 20 KB (4096 × 5) |

Cada stack se define como un array estático con alineación a 16 bytes (`#[repr(align(16))]`), requerida por la ABI de x86_64:

```rust
#[repr(align(16))]
struct AlignedStack([u8; 4096 * 5]);
static mut STACK: AlignedStack = AlignedStack([0; 4096 * 5]);
```

> **Nota:** Se usa `&raw const STACK` para obtener el puntero sin crear una referencia al `static mut`, evitando problemas de aliasing.

El stack pointer en la IST apunta al **tope** del stack (stack_start + STACK_SIZE), porque en x86 el stack crece hacia abajo.

---

## Entradas de la GDT

```rust
let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
let tss_selector  = gdt.add_entry(Descriptor::tss_segment(&TSS));
```

Tres entradas:
1. **Segmento de código del kernel** — necesario para long mode
2. **Segmento de datos del kernel** — necesario para `SS` válido
3. **Segmento TSS** — apunta al TSS que contiene la IST

---

## Inicialización (`gdt::init()`)

```rust
pub fn init() {
    GDT.0.load();           // Cargar la GDT con lgdt
    unsafe {
        CS::set_reg(GDT.1.code_selector);  // Recargar CS
        SS::set_reg(GDT.1.data_selector);  // Cargar SS con segmento de datos
        load_tss(GDT.1.tss_selector);      // Cargar TSS con ltr
    }
}
```

> **Orden:** `CS` se recarga con un far jump implícito. `SS` se establece al segmento de datos. `load_tss` ejecuta la instrucción `ltr` que le dice al CPU dónde está el TSS.

---

## Decisiones de diseño

- **20 KB por stack:** Tamaño conservador pero suficiente para handlers de excepción que no hacen recursión profunda.
- **Stacks separados para double fault y breakpoint:** Aisla las excepciones críticas de las de debugging.
- **`lazy_static!`:** Tanto TSS como GDT necesitan inicialización en runtime (por los punteros a los stacks estáticos).
