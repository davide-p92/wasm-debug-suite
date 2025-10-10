#include "wasm_debugger.hpp"
//#include <wasmtime.h>
#include <cstring>
#include <unordered_set>
#include <iostream>
#include <fstream>
#include <vector>


struct WasmDebugger::Impl {
	wasm_engine_t* engine = nullptr;
	wasmtime_store_t* store = nullptr;
	wasmtime_context_t* ctx = nullptr;
	wasmtime_module_t* module = nullptr;
	wasmtime_instance_t instance;
	wasmtime_memory_t memory{};
	bool has_mem = false;

	// Hostfunktion für "__trace(i32)"
	WasmDebugger* self = nullptr;
};

wasm_trap_t* host_trace(void* env, wasmtime_caller_t* caller, const wasmtime_val_t* args, size_t nargs, wasmtime_val_t* results, size_t nresults) {
	(void)caller; (void)results; (void)nresults;
	auto* self = reinterpret_cast<WasmDebugger*>(env);
	if (nargs >= 1 && args[0].kind == WASMTIME_I32) {
		uint32_t pc = args[0].of.i32;
		self->onTrace([&](const DebugEvent&){});
		self->handleTrace(pc);
	}
	return nullptr;
}

bool WasmDebugger::load(const std::string& path, std::string* err) {
	impl_ = new Impl();
	impl_->self = this;
	// Engine/Store anlegen (Wasmtime-API)
	impl_->engine = wasm_engine_new();
	impl_->store = wasmtime_store_new(impl_->engine, nullptr, nullptr);
	impl_->ctx = wasmtime_store_context(impl_->store);

	// module
	std::ifstream ifs(path, std::ios::binary);
	if (!ifs) { if (err) *err = "open wasm failed"; return false; }
	std::vector<uint8_t> bytes((std::istreambuf_iterator<char>(ifs)), {});
	wasmtime_error_t* werr = wasmtime_module_new(impl_->engine, bytes.data(), bytes.size(), &impl_->module);
	if (werr) { if (err) *err = "compile failed"; wasmtime_error_delete(werr); return false; }

	// linker + import "__trace(i32)"(direkt über define_func)
	wasmtime_linker_t* linker = wasmtime_linker_new(impl_->engine);
	wasm_functype_t* fty = wasm_functype_new_1_0(wasm_valtype_new_i32());
	 // define_func übernimmt ownership NICHT; wir löschen fty später manuell.
	 wasmtime_error_t* derr = wasmtime_linker_define_func(linker,
	 			"env", 3, "__trace", 7, fty /*Signatur (i32) -> ()*/, host_trace, this, nullptr);
	 wasm_functype_delete(fty);
	 if (derr) {
	 	if (err) *err = "define __trace failed";
	 	wasmtime_error_delete(derr);
	 	wasmtime_linker_delete(linker);
	 	return false;
	}
	// Instanz erzeugen
	wasm_trap_t* trap = nullptr;
	wasmtime_error_t* err2 = wasmtime_linker_instantiate(linker, impl_->ctx, impl_->module, &impl_->instance, &trap /*statt nullptr*/);
	//wasmtime_linker_delete(linker);
	if (err2) { /*if (err) *err = "instantiate failed"; 		wasmtime_error_delete(err2); std::cout << "Inside err2\n"; wasmtime_linker_delete(linker);*/
		wasm_name_t msg;
    		wasmtime_error_message(err2, &msg);
    		if (err) {
        		*err = std::string("instantiate failed: ") +
               		std::string(msg.data, msg.size);
    		}
		wasm_name_delete(&msg);
		wasmtime_error_delete(err2);
		return false; 
	}
	if (trap) {
    		if (err) *err = "trap occurred during instantiation";
    		wasm_trap_delete(trap);
    		return false;
	}

	// memory (nehme 0 an)
	wasmtime_extern_t item;
	bool ok = wasmtime_instance_export_get(impl_->ctx, &impl_->instance, "memory", 6, &item);
	if (ok && item.kind == WASMTIME_EXTERN_MEMORY) {
		impl_->memory = item.of.memory;
		impl_->has_mem = true;
	}
	return true;
}

void WasmDebugger::addBreakpoint(uint32_t pc) {
	if (std::find(breakpoints_.begin(), breakpoints_.end(), pc) == breakpoints_.end())
		breakpoints_.push_back(pc);
}
void WasmDebugger::clearBreakpoint(uint32_t pc) {
	breakpoints_.erase(std::remove(breakpoints_.begin(), breakpoints_.end(), pc), breakpoints_.end());
}

void WasmDebugger::handleTrace(uint32_t pc) {
	if (trace_cb_) trace_cb_({pc, ""});
	// "Step" endet am nächsten Trace; „Run“ bricht hier ab, wenn BP getroffen:
    // (Die Kontrolle bleibt beim Host, weil das Callback im Host läuft.)
    (void)pc;
}

bool WasmDebugger::run(const std::string& entry_func, std::string* err) {
	wasmtime_extern_t ext;
	bool ok = wasmtime_instance_export_get(impl_->ctx, &impl_->instance, entry_func.c_str(), entry_func.size(), &ext);
	if (!ok || ext.kind != WASMTIME_EXTERN_FUNC) { if (err) *err = "export not found"; return false; }
	wasmtime_val_t results[1];
	wasmtime_val_t params[0];
	wasm_trap_t* trap = nullptr;
	wasmtime_error_t* e = wasmtime_func_call(impl_->ctx, &ext.of.func, params, 0, results, 0, &trap);
	if (trap) { wasm_trap_delete(trap); if (err) *err = "trap"; return false; }
	if (e) { wasmtime_error_delete(e); if (err) *err = "call failed"; return false; }
	return true;
}

bool WasmDebugger::step(std::string* /*err*/) {
    // "step" ist modelliert als: rufe eine Exportfunktion auf, die genau einen __trace auslöst.
    // In der Praxis: Gestaltet man entry_func so, dass sie pro Call nur einen Block ausführt.
    return true;
}

bool WasmDebugger::readMemory(uint32_t offset, uint32_t size, std::vector<uint8_t>& out) {
	if (!impl_->has_mem) return false;
	size_t len;
	uint8_t* data = wasmtime_memory_data(impl_->ctx, &impl_->memory);
	len = wasmtime_memory_data_size(impl_->ctx, &impl_->memory);
	if (offset + size > len) return false;
	out.assign(data + offset, data + offset + size);
	return true;
}

int main() {
    WasmDebugger dbg;
    std::string err;
    if (!dbg.load("./samples/wasm_trace_demo.wasm", &err)) { std::cerr << err << "\n"; return 1; }
    dbg.onTrace([](const DebugEvent& e){ std::cout << "trace pc=" << e.pc << "\n"; });

    if (!dbg.run("main", &err)) { std::cerr << err << "\n"; return 1; }

    std::vector<uint8_t> buf;
    if (dbg.readMemory(0, 64, buf)) {
        for (auto b: buf) std::printf("%02X ", b);
        std::puts("");
    }
    return 0;
}
