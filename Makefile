# Makefile — Comandos útiles para Mucking Around

.DEFAULT_GOAL := help
.PHONY: help run cli check test clippy lint fmt fmt-fix web-build web-serve web clean reset-save

# Colores para salida de terminal
CYAN  := \033[0;36m
GREEN := \033[0;32m
RESET := \033[0m

help: ## Muestra este menú de ayuda con los comandos disponibles
	@echo ""
	@echo "$(CYAN)Mucking Around — Comandos de Desarrollo:$(RESET)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(RESET) %s\n", $$1, $$2}'
	@echo ""

run: ## Inicia el juego en la terminal (CLI nativo)
	cargo run -p cli

cli: run ## Alias para ejecutar la terminal nativa

check: ## Verifica la compilación de todo el workspace
	cargo check --workspace

test: ## Ejecuta la suite completa de pruebas unitarias
	cargo test --workspace

clippy: ## Ejecuta el linter Clippy verificando advertencias
	cargo clippy --workspace -- -D warnings

lint: clippy ## Alias para clippy

fmt: ## Verifica el formato del código según rustfmt
	cargo fmt --all -- --check

fmt-fix: ## Aplica el formato automático a todo el código
	cargo fmt --all

web-build: ## Compila el crate web a WebAssembly con wasm-pack
	wasm-pack build crates/web --target web --out-dir ../../www/pkg

web-serve: ## Levanta un servidor local en el puerto 8080 para la versión web
	@echo "Iniciando servidor en http://localhost:8080 ..."
	python3 -m http.server 8080 --directory www

web: web-build ## Compila WASM y levanta el servidor web local
	$(MAKE) web-serve

clean: ## Limpia los artefactos de compilación (target y www/pkg)
	cargo clean
	rm -rf www/pkg

reset-save: ## Elimina save.json para reiniciar la partida de la terminal
	rm -f save.json
	@echo "save.json eliminado. La próxima partida CLI iniciará limpia."
