# kur-os 🦀 🖥️

Un kernel de 64 bits desarrollado en Rust, explorando el "Abzu" de los sistemas operativos.

kur-os es un proyecto educativo enfocado en la implementación de un sistema operativo desde cero (bare-metal) para la arquitectura x86_64. Este proyecto sirve como base práctica para entender la gestión de memoria, interrupciones y la comunicación con el hardware sin una capa intermedia.

## 🚀 Características Actuales

| Componente | Estado | Descripción |
|------------|--------|-------------|
| VGA Buffer | ✅ Funcional | Driver para salida de texto con soporte de colores y scroll |
| Serial Port | ✅ Funcional | Comunicación vía UART para debugging en la terminal del host |
| GDT/TSS | ✅ Funcional | Global Descriptor Table con Task State Segment para stacks de excepciones |
| IDT | ✅ Funcional | Interrupt Descriptor Table con handlers para breakpoint y double fault |
| Testing Framework | ✅ Funcional | Sistema de pruebas unitarias e integración en QEMU |

## 🏗️ Arquitectura del Proyecto

```text
src/
├── lib.rs          # Núcleo del kernel, expone módulos y función init()
├── main.rs         # Punto de entrada (_start)
├── gdt.rs          # Global Descriptor Table y Task State Segment
├── interrupts.rs   # Interrupt Descriptor Table y handlers de excepciones
├── vga_buffer.rs   # Driver para el buffer VGA en modo texto
└── serial.rs       # Driver para el puerto serie COM1
```

## ⚙️ Target Custom

El kernel usa un target personalizado (`x86_64-kur_os.json`) con configuraciones especiales:

| Opción | Valor | Propósito |
|--------|-------|-----------|
| `rustc-abi` | `x86-softfloat` | Evita instrucciones SSE en handlers de interrupción |
| `disable-redzone` | `true` | Necesario para código de kernel (la red zone causaría corrupción) |
| `panic-strategy` | `abort` | No hay stack unwinding en bare-metal |
| `features` | `-mmx,-sse,-sse2,+soft-float` | Deshabilita SIMD, usa emulación de floats |

**Nota**: El uso de `soft-float` significa que cualquier operación de punto flotante será emulada en software. Esto es aceptable para un kernel educativo ya que el código del kernel raramente usa floats.

## 🛠️ Requisitos Previos

1. **Rust Nightly**:
   ```bash
   rustup override set nightly
   ```

2. **Componentes adicionales**:
   ```bash
   rustup component add rust-src llvm-tools-preview
   ```

3. **Bootimage**:
   ```bash
   cargo install bootimage
   ```

4. **QEMU**: Asegurate de tener `qemu-system-x86_64` instalado.

## 🔧 Ejecución y Testing

### Correr el Kernel

```bash
cargo run
```

Esto compila el kernel, crea una imagen booteable y la lanza en QEMU.

### Ejecutar Pruebas

```bash
cargo test
```

Ejecuta:
- Unit tests en la biblioteca
- Integration tests (`basic_boot.rs`)
- Negative testing (`should_panic.rs`)

## 🎓 Contexto Académico

Este proyecto es desarrollado por Lautaro, estudiante de la Licenciatura en Ciencias de la Computación en la Universidad Nacional del Sur (UNS). El objetivo es profundizar en conceptos de arquitectura de computadoras y sistemas operativos, sirviendo de base para una investigación sobre la eficiencia en la gestión de memoria en lenguajes de sistemas modernos.

## 📜 Licencia

Este proyecto se distribuye bajo la licencia MIT. Consultá el archivo LICENSE para más detalles.