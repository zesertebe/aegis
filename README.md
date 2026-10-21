# aegis

[![License: GPL v3](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](LICENSE)
![Version](https://img.shields.io/badge/version-0.1.0-green.svg)

**aegis** es un gestor de bóvedas cifradas para la terminal, construido sobre
[restic](https://restic.net). Administra múltiples bóvedas de respaldo (personal,
trabajo, notas…) desde una TUI minimalista y orientada a teclado, sin salir
del terminal.

## Características

- **Múltiples bóvedas** con CRUD completo desde la propia TUI (crear, editar, eliminar).
- **Push / Pull / History / Init / Mount** sobre cualquier repo que soporte restic (local, SFTP, B2, S3…).
- **100% teclado, estilo vim**: `j`/`k` para navegar, atajos directos (`b`, `u`, `h`…), atajos siempre visibles en la pie de pantalla.
- **Config en JSON** legible y editable a mano en `~/.config/aegis/config.json`.
- **Headless**: `aegis list` y `aegis check` para scripts y comprobaciones rápidas.
- **Sin crashes**: si `restic` no está instalado, la TUI lo indica en pantalla y sigue siendo navegable.
- Binario único y estático en comportamiento: `cargo build --release` y ya.

## Captura

![aegis TUI](docs/screenshot.gif)
_(GIF pendiente)_

## Requisitos

- [`restic`](https://restic.net) en el PATH — Arch: `sudo pacman -S restic`
- `fuse3` solo si quieres usar `Mount` — Arch: `sudo pacman -S fuse3`

## Instalación

Desde el repositorio:

```sh
git clone https://github.com/zesertebe/aegis
cd aegis
cargo build --release
# el binario queda en target/release/aegis
```

O directamente con cargo (sin clonar):

```sh
cargo install --git https://github.com/zesertebe/aegis
```

## Uso

```sh
aegis          # abre la TUI
aegis list     # lista las bóvedas configuradas (headless)
aegis check    # comprueba que restic esté instalado
```

La configuración vive en `~/.config/aegis/config.json`. Ejemplo:

```json
{
  "vaults": [
    {
      "name": "Personal",
      "local_path": "~/Notas",
      "repo_target": "/mnt/nube/backups"
    }
  ]
}
```

### Atajos de la TUI

| Pantalla | Atajos |
|---|---|
| Lista de bóvedas | `Enter` abrir · `n` nueva · `e` editar · `d` eliminar · `q` salir |
| Menú de bóveda | `b` push · `u` pull · `h` history · `i` init · `m` mount · `U` unmount · `Esc` atrás |
| Snapshots | `↑`/`↓` o `j`/`k` navegar · `Esc` atrás |
| Popups | `Enter` confirmar · `Esc` cancelar |

`Ctrl-C` / `Ctrl-Q` sale siempre.

## Seguridad

**Cómo protege tus datos:**

- **La password de restic nunca se guarda en disco.** La configuración de aegis
  solo contiene rutas; el password se pide por un prompt enmascarado (`***`) en
  cada operación y se descarta al terminar.
- **Nunca aparece en la línea de comandos.** Se pasa al proceso `restic` como
  variable de entorno (`RESTIC_PASSWORD`), no como argumento, así que no es
  visible en `ps` ni en el historial de la shell.
- **Sin inyección de comandos.** aegis invoca `restic` con argumentos directos
  (`std::process::Command`), sin pasar por un shell.
- **aegis no usa la red.** No abre conexiones ni envía nada: el transporte y el
  cifrado de los respaldos los lleva íntegramente restic (AES-256 en reposo,
  TLS según el backend).
- **Acciones destructivas con confirmación.** `Pull` avisa de qué ruta va a
  sobrescribir antes de restaurar, y `Delete` pide confirmación `y/N`.
- **Config local con permisos de tu usuario**, sin secretos dentro.

**Limitaciones conocidas:**

- El password vive en memoria RAM (`String`) durante la operación; no se limpia
  con `zeroize`, por lo que podría quedar en un core dump o en swap. Para
  bóvedas de alta sensibilidad, considera un repositorio con `restic` + llave
  (`RESTIC_PASSWORD_FILE`) — por ahora aegis no lo soporta.
- `Pull` restaura el snapshot `latest` sobre las rutas originales: está pensado
  como recuperación, no como exploración versionada (vía `Roadmap`).

## UX y limitaciones

- **Operaciones síncronas:** mientras `restic` trabaja (push/pull), la TUI queda
  en pausa y muestra "Ejecutando…"; no hay barra de progreso en vivo. El
  resultado se muestra en un overlay con la salida completa de restic
  (stdout + stderr) y se puede hacer scroll.
- **Errores legibles:** cualquier fallo de restic (repo no inicializado,
  password incorrecta, red caída) aparece en pantalla con el mensaje real del
  comando, nunca un fallo silencioso.
- **Estado siempre visible:** barra de estado inferior con el resultado de la
  última operación, indicador `●` sobre la bóveda montada, y versión de restic
  en la cabecera.

## Roadmap

- [ ] Barra de progreso en vivo de `restic` (leer `--json` en un hilo aparte).
- [ ] Elegir snapshot concreto al restaurar (hoy solo `latest`).
- [ ] Soporte `RESTIC_PASSWORD_FILE` / clave del sistema.
- [ ] Operaciones en segundo plano sin bloquear la TUI.
- [ ] Tests unitarios (`config`/`restic`) con target `lib`.

## Contribuir

Issues y PRs bienvenidos en
[github.com/zesertebe/aegis](https://github.com/zesertebe/aegis).

- `cargo fmt --check` y `cargo clippy -- -D warnings` deben pasar antes del PR.
- El código se organiza en módulos planos bajo `src/` (`config`, `restic`,
  `app`, `ui`, `input`, `cli`); respeta esa estructura.

## Licencia

[GPL-3.0](LICENSE).
