# kur-os 🦀 🖥️

Un kernel de 64 bits desarrollado en ```Rust```, explorando el "Abzu" de los sistemas operativos.

kur-os es un proyecto educativo enfocado en la implementación de un sistema operativo desde cero (bare-metal) para la arquitectura x86_64. Este proyecto sirve como base práctica para entender la gestión de memoria, interrupciones y la comunicación con el hardware sin una capa intermedia.

## 🚀 Características Actuales
Componente	Estado	Descripción
- VGA Buffer	✅ Funcional	Driver para salida de texto con soporte de colores y scroll.
- Serial Port	✅ Funcional	Comunicación vía UART para debugging en la terminal del host.
- Testing Framework	✅ Funcional	Sistema de pruebas unitarias e integración en QEMU.
- Modularidad	✅ Implementado	Separación clara entre lib.rs y el punto de entrada main.rs.
- IDT	🛠️ En desarrollo	Preparando la Tabla de Descriptores de Interrupción.

## 🏗️ Arquitectura del Proyecto

El proyecto sigue una estructura modular para garantizar la seguridad de memoria y la facilidad de testeo:

- src/lib.rs: El núcleo del sistema, exporta drivers y utilidades de bajo nivel.
- src/vga_buffer.rs: Implementación segura de la interfaz de video usando el crate volatile.
- src/serial.rs: Driver para el puerto serie COM1.
- tests/: Pruebas de integración independientes que corren en sus propios entornos de QEMU.

## 🛠️ Requisitos previos

Para compilar y correr kur-os, necesitás el canal nightly de Rust debido al uso de características inestables del compilador.

- Rust Nightly:
    Bash
    rustup override set nightly

- Bootimage:
    Bash
    cargo install bootimage

- QEMU: Asegurate de tener qemu-system-x86_64 instalado en tu sistema.

## 🔧 Ejecución y Testing
Correr el Kernel

Para compilar y lanzar el kernel en una máquina virtual QEMU:
- Bash

cargo run

Ejecutar Pruebas

El proyecto utiliza un Test Runner personalizado para reportar resultados directamente en la consola del host:
- Bash

cargo test

Esto ejecutará:
- Unit Tests en la biblioteca.
- Integration Tests (como basic_boot.rs).
- Negative Testing (vía should_panic.rs).

## 🎓 Contexto Académico

Este proyecto es desarrollado por Lautaro, estudiante de la Licenciatura en Ciencias de la Computación en la Universidad Nacional del Sur (UNS). El objetivo es profundizar en conceptos de arquitectura de computadoras y sistemas operativos, sirviendo de base para una investigación sobre la eficiencia en la gestión de memoria en lenguajes de sistemas modernos.

## 📜 Licencia
Este proyecto se distribuye bajo la licencia MIT. Consultá el archivo LICENSE para más detalles