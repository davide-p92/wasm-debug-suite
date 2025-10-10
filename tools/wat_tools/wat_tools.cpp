#include "wabt/binary-reader-ir.h"
#include "wabt/binary-writer.h"
#include "wabt/feature.h"
#include "wabt/ir.h"
#include "wabt/stream.h"
#include "wabt/validator.h"
#include "wabt/wast-parser.h"
#include "wabt/wat-writer.h"
#include "wabt/error-formatter.h"
//#include "wabt/wasm2wat.h"

#include <vector>
#include <fstream>
#include <iostream>
#include <string>
#include <memory>
#include <cstring>
#include <cstdio>

using namespace wabt;

static bool HasErrors(const Errors& errors) {
  for (const Error& error : errors) {
    if (error.error_level == ErrorLevel::Error) {
      return true;
    }
  }
  return false;
}

static std::string DefaultOutputName(const std::string& input, const std::string& extension) {
	size_t last_dot = input.find_last_of('.');
	if (last_dot == std::string::npos) {
		return input + extension;
	}
	return input.substr(0, last_dot) + extension;
}

// Options for wat2wasm command
struct Wat2WasmOptions {
	std::string input_filename;
	std::string output_filename;
	Features features;
	bool debug_parser = false;
	bool dump_module = false;
	bool relocatable = false;
	bool no_canonicalize_lebs = false;
	bool debug_names = false;
	bool no_check = false;
	int verbose = 0;
};

static int wat2wasm(const std::string& in, const std::string& out) {
	// Lies Quelltext ein
	std::ifstream ifs(in);
	if (!ifs) {
	fprintf(stderr, "Konnte Datei %s nicht öffnen\n", in.c_str());
	return 1;
	}
	std::string source((std::istreambuf_iterator<char>(ifs)),
		        std::istreambuf_iterator<char>());

	Features features;
	features.EnableAll();

	Errors errors;
	std::unique_ptr<Module> module;

	// WAT parsen: Lexer erzeugen
	auto lexer = WastLexer::CreateBufferLexer(
		in, source.data(), source.size(), &errors
	);
	// Parser erstellen mit Optionen
	WastParseOptions parse_opts(features);
	WastParser parser(lexer.get(), &errors, &parse_opts);
	
	if (Failed(parser.ParseModule(&module))) {
		FormatErrorsToFile(errors, Location::Type::Text);
		return 2;
	}
	
	// Modul Validieren
	if (Failed(ValidateModule(module.get(), &errors, features))) {
		FormatErrorsToFile(errors, Location::Type::Text);
		return 3;
	}

	// Binär ausgeben
	//OutputBuffer outbuf;
	MemoryStream outbuf;
	//Optionen setzen
	WriteBinaryOptions write_opts(features,
			true, // canonicalize_lebs
			false, // relocatable
			true); // wrie_debug_names
	// Schreiben
	if (Failed(WriteBinaryModule(&outbuf, module.get(), write_opts))) {
		fprintf(stderr, "Fehler beim Schreiben des Binärmoduls\n");
		return 4;
	}

	outbuf.output_buffer().WriteToFile(out);
	//std::ofstream ofs(out, std::ios::binary);
	//ofs.write(reinterpret_cast<const char*>(outbuf.data.data()), outbuf.data.size());

	return 0;
}

static int wasm2wat(const std::string& in, const std::string& out) {
	Errors errors;
	std::ifstream ifs(in, std::ios::binary);
	if (!ifs) {
		std::cerr << "Error: cannot open input file " << in << std::endl;
		return 1;
	}
	std::vector<uint8_t> bin((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());
	
	Module module;
	ReadBinaryOptions read_options;
	read_options.features = Features();
	read_options.read_debug_names = true;
	
	ReadBinaryIr(in.c_str(), bin.data(), bin.size(), read_options, &errors, &module);
	if(!errors.empty()) {
		std::cerr << FormatErrorsToString(errors, wabt::Location::Type::Binary);
		return 1;
	}
	
	MemoryStream stream;
	WriteWatOptions write_options;
	WriteWat(&stream, &module, write_options);
	
	std::ofstream ofs(out);
	if (!ofs) {
		std::cerr << "Error: cannot open output file " << out << std::endl;
		return 1;
	}
	ofs.write(reinterpret_cast<const char*>(stream.output_buffer().data.data()), stream.output_buffer().data.size());
	
	return 0;
}

int main(int argc, char** argv) {
	if (argc < 4) { std::cerr << "usage: wat_tools (wat2wasm|wasm2wat) <in> <out>\n"; return 1; }
	std::string cmd = argv[1], in = argv[2], out = argv[3];
	if (cmd == "wat2wasm") { return wat2wasm(in, out); }
	else if (cmd == "wasm2wat") { return wasm2wat(in, out); }
	else {
		std::cerr << "Unknown command: " << cmd << "\n";
		return 1;
	}

}
