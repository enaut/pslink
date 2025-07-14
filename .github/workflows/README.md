# GitHub Actions Workflow-Aufteilung

## Übersicht

Die Docker Build-Workflows wurden in drei separate Dateien aufgeteilt, um die Build-Performance zu verbessern:

### 1. `build_docker_x86_64.yml`
- Baut das Docker-Image für x86_64/AMD64 Architektur
- Läuft auf standard Ubuntu Runnern (schnell)
- Erstellt Tags: `x86_64-latest` und `x86_64-<version>`

### 2. `build_docker_arm64.yml`
- Baut das Docker-Image für ARM64/AArch64 Architektur
- Läuft auf emulierten ARM64 Runnern (langsamer, aber parallel)
- Erstellt Tags: `aarch64-latest` und `aarch64-<version>`

### 3. `create_multiarch_image.yml`
- Wartet auf beide Architecture-Builds
- Erstellt Multi-Architektur Manifests
- Kombiniert beide Images zu: `latest` und `<version>` Tags

## Vorteile

- **Parallelisierung**: x86_64 und ARM64 Builds laufen gleichzeitig
- **Effizienz**: Schnellere x86_64 Builds blockieren nicht die ARM64 Builds
- **Modularität**: Einzelne Workflows können unabhängig getestet werden
- **Flexibilität**: Einzelne Architekturen können bei Bedarf übersprungen werden

## Trigger

Alle Workflows werden ausgelöst durch:
- Push auf Tags (z.B. `v1.0.0`)
- Manueller workflow_dispatch

Das Multi-Arch Image wird nur erstellt, wenn beide Architecture-Builds erfolgreich sind.


## Troubleshooting

- Falls ein Architecture-Build fehlschlägt, wird kein Multi-Arch Image erstellt
- Einzelne Architecture-Images sind trotzdem verfügbar
- Workflows können manuell über GitHub Actions UI ausgelöst werden
