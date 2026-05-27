# Makefile for Analytics Server

PYTHON := python3
VENV := venv
VENV_BIN := $(VENV)/bin

.PHONY: help codegen install run test clean venv

help:
	@echo "Analytics Server - Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  venv     - Create Python virtual environment"
	@echo "  codegen  - Generate Python code from protobuf"
	@echo "  install  - Install Python dependencies"
	@echo "  run      - Run the analytics server"
	@echo "  test     - Run tests"
	@echo "  clean    - Clean generated files"

venv:
	@echo "Creating virtual environment..."
	$(PYTHON) -m venv $(VENV)
	@echo "Virtual environment created at $(VENV)"
	@echo "Activate with: source $(VENV_BIN)/activate"

codegen:
	@echo "Generating Python protobuf code..."
	$(PYTHON) -m grpc_tools.protoc \
		-I../../proto \
		--python_out=. \
		--grpc_python_out=. \
		--pyi_out=. \
		../../proto/analytics.proto
	@sed -i.bak 's/^import analytics_pb2 as analytics__pb2$$/from analytics_server import analytics_pb2 as analytics__pb2/' analytics_pb2_grpc.py && rm -f analytics_pb2_grpc.py.bak
	@echo "Protobuf code generated successfully"

install: venv
	@echo "Installing dependencies..."
	$(VENV_BIN)/pip install -r requirements.txt
	@echo "Dependencies installed"

run:
	@echo "Starting analytics server..."
	cd ../.. && PYTHONPATH=svcs $(PYTHON) -m analytics_server

test:
	@echo "Running tests..."
	$(PYTHON) -m pytest tests/ -v

clean:
	@echo "Cleaning generated files..."
	rm -f analytics_pb2.py analytics_pb2_grpc.py analytics_pb2.pyi
	rm -rf __pycache__ .pytest_cache $(VENV)
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	@echo "Clean complete"
