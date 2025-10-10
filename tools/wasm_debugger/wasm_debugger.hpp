#pragma once
#include <cstdint>
#include <functional>
#include <string>
#include <vector>
#include <wasmtime.h>

struct DebugEvent {
	uint32_t pc;
	std::string label;
};

class WasmDebugger {
public:
	using TraceCallback = std::function<void(const DebugEvent&)>;
	// lädt .wasm, bereitet Import "__trace(i32)" vor
	bool load(const std::string& path, std::string* err);

	// setz/fall Breakpoint
	void addBreakpoint(uint32_t pc_or_hash);
	void clearBreakpoint(uint32_t pc_or_hash);

	// run/step
	bool run(const std::string& entry_func, std::string* err);
	bool step(std::string* err);

	// memory
	bool readMemory(uint32_t offset, uint32_t size, std::vector<uint8_t>& out);
	// trace callback
	void onTrace(TraceCallback cb) { trace_cb_ = std::move(cb); }
	
	// friend um handleTrace benutzen zu können
	friend wasm_trap_t* host_trace(void*, wasmtime_caller_t*,
                                   const wasmtime_val_t*, size_t,
                                   wasmtime_val_t*, size_t);

private:
	void handleTrace(uint32_t pc);
	TraceCallback trace_cb_;
	std::vector<uint32_t> breakpoints_;

	// Wasmtime handles
	struct Impl;
	Impl* impl_ = nullptr;
};
