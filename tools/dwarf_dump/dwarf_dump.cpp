// dwarf_dump.cpp
// tools/dwarf_dump/dwarf_dump.cpp

#include <llvm/Object/ObjectFile.h>
#include <llvm/Object/Wasm.h>
#include <llvm/DebugInfo/DWARF/DWARFContext.h>
#include <llvm/DebugInfo/DWARF/DWARFDie.h>
#include <llvm/DebugInfo/DWARF/DWARFExpression.h>
#include <llvm/DebugInfo/DWARF/DWARFUnit.h>
#include <llvm/DebugInfo/DWARF/DWARFDebugLoc.h>
#include <llvm/DebugInfo/DWARF/DWARFFormValue.h>
#include <llvm/Support/MemoryBuffer.h>
#include <llvm/Support/Error.h>
#include <llvm/Support/raw_ostream.h>

#include "./external/json/single_include/nlohmann/json.hpp"
#include <iostream>
#include <fstream>
#include <string>
#include <optional>
#include <iomanip>

using json = nlohmann::json;
using namespace llvm;
using namespace llvm::object;

static std::optional<std::string> getName(const DWARFDie& die) {
	if (!die.isValid()) return std::nullopt;
	if (auto attr = die.find(llvm::dwarf::DW_AT_name)) {
		if (auto s = attr->getAsCString()) return std::string(*s);
	}
	return std::nullopt;
}

static std::optional<uint64_t> getUData(const DWARFDie& die, dwarf::Attribute A) {
	if (!die.isValid()) return std::nullopt;
	if (auto attr = die.find(A)) {
		if (auto v = attr->getAsUnsignedConstant()) return *v;
	}
	return std::nullopt;
}

static std::string resolveTypeName(DWARFDie die) {
	if (!die.isValid()) return "<invalid>";	
	// Rekursiv DW_AT_type folgen bis Namen vorhanden
	for (int  depth = 0; depth < 32 && die; ++depth) {
		if (auto n = getName(die)) return *n;
		if (auto attr = die.find(llvm::dwarf::DW_AT_type)) {
			if (auto ref = attr->getAsReference()) {
				die = die.getDwarfUnit()->getDIEForOffset(ref.value());
				continue;
			}
		}
		break;
	}
	return "<unknown>";
}

int main(int argc, char** argv) {
	if (argc < 3) {
		errs() << "usage: dwarf_dump <input.wasm> <out.json>\n";
		return 1;
	}
	
	// 1. Datei einlesen
	std::string in = argv[1], out = argv[2];

	std::ofstream debug_log("dwarf_debug.log");
	debug_log << "===== DWARF DEBUG LOG =====" << std::endl;
	auto bufOrErr = MemoryBuffer::getFile(in);
	if (!bufOrErr) { errs() << "open failed\n"; return 1; }
	std::unique_ptr<DWARFContext> ctx;
	std::unique_ptr<object::ObjectFile> obj;
	std::unique_ptr<object::WasmObjectFile> wasmObj;
	
	// 2. Objektdatei erstellen
	if(auto objOrErr = ObjectFile::createObjectFile(bufOrErr->get()->getMemBufferRef())) {
		obj = std::move(*objOrErr);
		ctx = DWARFContext::create(*obj);
	} else { 
		//errs() << "not an object\n"; return 1; }
		// Spezieller Workaround für wasm-bindgen Dateien
		auto wasmObjOrErr = object::WasmObjectFile::createWasmObjectFile(
			bufOrErr->get()->getMemBufferRef());
		if(!wasmObjOrErr) {
			errs() << "Error creating object: " << toString(wasmObjOrErr.takeError()) << "\n";
			return 1;
		}
		wasmObj = std::move(*wasmObjOrErr);
		ctx = DWARFContext::create(*wasmObj);
	}
		if (!ctx) {
			errs() << "Failed to create DWARF context\n";
			return 1;
		}
		
		//DWARF Statistiken sammeln
		debug_log << "Number of Compilation Units: " << ctx ->getNumCompileUnits() << std::endl;
		debug_log << "DWARF Sections: " << std::endl;
		/*debug_log << "  .debug_info size: " << ctx->getDWARFObj().getInfoSection().Data.size() << std::endl;
		debug_log << "  .debug_abbrev size: " << ctx->getDWARFObj().getAbbrevSection().Data.size() << std::endl;
		debug_log << "  .debug_str size: " << ctx->getDWARFObj().getStringSection().Data.size() << std::endl;
		debug_log << "  .debug_loc size: " << ctx->getDWARFObj().getLocSection().Data.size() << std::endl;
		debug_log << "  .debug_loclists size: " << ctx->getDWARFObj().getLoclistsSection().Data.size() << std::endl;
		*/
		json registry = { 
			{"types", json::object()}, 
			{"variables", json::array()},
			{"functions", json::array()}
		};
		
		// Detaillierte Debug-Informationen für jede CU
		int cu_index = 0;
		std::string cu_name = "<noname>";
		for (auto& cu : ctx->compile_units()) {
			if (auto name_opt = getName(cu->getUnitDIE()))
				cu_name = *name_opt;
			json cu_info = {
				{"index", cu_index},
				{"offset", cu->getOffset()},
				{"version", cu->getVersion()},
				{"address_size", cu->getAddressByteSize()},
				{"name", cu_name},
				{"dies", json::array()}
			};
			
			debug_log << "\nCompilation Unit #" << cu_index << std::endl;
			debug_log << "  Name: " << cu_info["name"] << std::endl;
			debug_log << "  Offset: 0x" << std::hex << cu->getOffset() << std::dec << std::endl;
			debug_log << "  Version: " << cu->getVersion() << std::endl;
			debug_log << "  DIEs count: " << cu->getNumDIEs() << std::endl;
			
			// Rekursive Verarbeitung mit Debugging
			uint64_t die_count = 0;
			const DWARFUnitIndex &CUIndex = ctx->getCUIndex();
			const DWARFObject &dwarfObj = ctx->getDWARFObj();
			bool isLittleEndian = ctx->isLittleEndian();
			int struct_count = 0;
			int var_count = 0;
			int fun_count = 0;
			static int total_die_count = 0; // Debug globale Variable
			// 3. Rekursive Verarbeitungsfunktion
			// Tiefensuche über alle DIEs:
			std::function<void(DWARFDie, int)> dfs = [&](DWARFDie d, int depth) {
				if (!d.isValid()) return;
				die_count++;
				std::cout << "DIEs count: " << die_count << std::endl;
				auto tag = d.getTag();
				llvm::StringRef tagName = dwarf::TagString(tag);
				if (tagName == "DW_TAG_null") return;
				if (tagName.empty()) tagName = "UNKNOWN_TAG";

		    		llvm::outs() << "  " << std::string(depth * 2, ' ') 
				 	<< tagName << ": ";
		    		// Debug-Informationen
		    		json die_info = {
		    			{"offset", d.getOffset()},
		    			{"tag", tagName},
		    			{"depth", depth}
		    		};
		    		// Attribute sammeln
		    		json attributes = json::array();
		    		for (auto attr : d.attributes()) {
		    			json attr_info = {
		    				{"name", dwarf::AttributeString(attr.Attr)}
		    			};
		    			
		    			if (auto str = attr.Value.getAsCString()) {
		    				attr_info["value"] = *str;
		    			} else if (auto intVal = attr.Value.getAsUnsignedConstant()) {
		    				attr_info["value"] = *intVal;
		    			} //else if (auto ref = attr.Value.getAsReference()) {
		    				//attr_info["value"] = llvm::format("0x{0:x}", *ref);
		    			//} 
		    			else {
		    				attr_info["value"] = "unresolved";
		    				
		    			}
		    			attributes.push_back(attr_info);
		    		}
		    		die_info["attributes"] = attributes;
		    		
		    		// Zur CU-Info
		    		cu_info["dies"].push_back(die_info);
		    		
		    		//debug_log << std::string(depth * 2, ' ') << "DIE [" << die_count << "]: " << tagName << " (0x" << std::hex << d.getOffset() << std::dec << ")" << std::endl;
		    		
				if (auto name = getName(d)) {
					llvm::outs() << *name;
				} else {
					llvm::outs() << "<noname>";
				}

				llvm::outs() << " (0x" << llvm::Twine::utohexstr(d.getOffset()) << ")\n";
				if (tag == dwarf::DW_TAG_member) {
					struct_count++;
					auto name = getName(d).value_or("<anon>");
					auto size = getUData(d, dwarf::DW_AT_byte_size).value_or(0);
					std::cout << "tag: " << "\tname: " << name << std::endl;
					json fields = json::array();
					DWARFDie ch = d.getFirstChild();
					while (ch.isValid()) {
						total_die_count++;
						if(ch.getTag() == dwarf::DW_TAG_member) {
							auto fname = getName(ch).value_or("<field>");
							auto foff = getUData(ch, dwarf::DW_AT_data_member_location).value_or(0);
							std::cout << "foff: " << foff << std::endl;
							//Feldtyp auflösen
							std::string ftype = resolveTypeName(ch);
							fields.push_back(json{{"name", fname}, {"offset", foff}, {"type", ftype}});
						}
						ch = ch.getSibling();
					}
					registry["types"][name] = {{"size", size}, {"fields", fields}};
				} else if (tag == dwarf::DW_TAG_variable) {
					var_count++;
					auto vname = getName(d).value_or("<var>");
					uint64_t addr = 0;
					if (auto loc = d.find(dwarf::DW_AT_location)) {
						if (auto exprLoc = loc->getAsBlock()) {
							DWARFDataExtractor dataExtr(ArrayRef<uint8_t>(exprLoc->data(), exprLoc->size()), ctx->isLittleEndian(), d.getDwarfUnit()->getAddressByteSize());
							DWARFExpression expr(dataExtr, d.getDwarfUnit()->getAddressByteSize());
							for(const auto& op : expr) {
								if (op.getCode() == dwarf::DW_OP_addr) {
									addr = op.getRawOperand(0);
									break;
								}
							}
						}
					}
					registry["variables"].push_back({{"name", vname}, {"address", addr}});
				} else if (tag == dwarf::DW_TAG_subprogram) {
					fun_count++;
					auto fname = getName(d).value_or("<unknown_function>");
		    			uint64_t low_pc = 0;
					uint64_t high_pc = 0;
					std::string return_type = "<unknown>";
					json parameters = json::array();
					
					if (auto low_pc_attr = d.find(dwarf::DW_AT_low_pc))
						low_pc = low_pc_attr->getAsAddress().value_or(0);
					if (auto high_pc_attr = d.find(dwarf::DW_AT_high_pc))
						high_pc = high_pc_attr->getAsAddress().value_or(0);
					if (auto type_attr = d.find(dwarf::DW_AT_type)) {
						if (auto ref = type_attr->getAsReference()) {
							DWARFDie type_die = d.getDwarfUnit()->getDIEForOffset(*ref);
							return_type = resolveTypeName(type_die);
						}
					}
					
					DWARFDie ch = d.getFirstChild();
					while (ch.isValid()) {
						if (ch.getTag() == dwarf::DW_TAG_formal_parameter) {
							auto pname = getName(ch).value_or("<param>");
							std::string ptype = "<unknown>";
							if (auto type_attr = ch.find(dwarf::DW_AT_type)) {
								if (auto ref = type_attr->getAsReference()) {
									DWARFDie type_die = ch.getDwarfUnit()->getDIEForOffset(*ref);
									ptype = resolveTypeName(type_die);
								}
							}
							parameters.push_back(json{{"name", pname}, {"type", ptype}});
						}
						ch = ch.getSibling();
					}
					registry["functions"].push_back({
						{"name", fname},
						{"low_pc", low_pc},
						{"high_pc", high_pc},
						{"return_type", return_type},
						{"parameters", parameters}
					});
				} else if (tag == dwarf::DW_TAG_structure_type) {
					auto name = getName(d).value_or("anon_" + std::to_string(d.getOffset()));
					auto size = getUData(d, dwarf::DW_AT_byte_size).value_or(0);
					
					json fields = json::array();
					DWARFDie child = d.getFirstChild();
					while (child.isValid()) {
						if (child.getTag() == dwarf::DW_TAG_member) {
							auto fieldName = getName(child).value_or("<field>");
							auto offset = getUData(child, dwarf::DW_AT_data_member_location).value_or(0);
							std::string typeName = resolveTypeName(child);
							
							fields.push_back({
								{"name", fieldName},
								{"offset", offset},
								{"type", typeName}
							});
						}	
						child = child.getSibling();
					}
					
					registry["types"][name] = {
						{"kind", "struct"},
						{"size", size},
						{"fields", fields},
					};
				}
				else if (tag == dwarf::DW_TAG_enumeration_type) {
					auto name = getName(d).value_or("anon_" + std::to_string(d.getOffset()));
					json variants = json::array();
					
					DWARFDie child = d.getFirstChild();
					while (child.isValid()) {
						if (child.getTag() == dwarf::DW_TAG_enumerator) {
							auto varName = getName(child).value_or("<variant>");
							auto value = getUData(child, dwarf::DW_AT_const_value).value_or(0);
							
							variants.push_back({
								{"name", varName},
								{"value", value}
							});
						}
						child = child.getSibling();
					}
					registry["types"][name] = {
						{"kind", "enum"},
						{"variants", variants}
					};
				} else if (tag == dwarf::DW_TAG_typedef) {
					if (auto name = getName(d)) {
						std::string baseType = resolveTypeName(d);
						registry["types"][*name] = {
							{"kind", "typedef"},
							{"base", baseType}
						};
					}
				}
				else if (tag == dwarf::DW_TAG_pointer_type) {
					std::cout << "Etwas pointeres" << std::endl;
					if (auto name = getName(d)) {
						auto pname = name.value_or("<anon_" + std::to_string(d.getOffset()));
						std::cout << "pname: " << pname << std::endl;
						if (name->find("__wbindgen_") != std::string::npos) {
							registry["wasm_special"]["pointers"].push_back({
								{"name", *name},
								{"bindgen_type", "memory_pointer"}
							});
						}
					}
				}
				// Rekursion über Kinder
				DWARFDie child = d.getFirstChild();
				while (child.isValid()) {
					dfs(child, depth + 1);
					child = child.getSibling();
				}
			};
			
			// Compilation Units verwenden
			for (auto& cu : ctx->compile_units()) {
				DWARFDie cuDie = cu->getUnitDIE();
				if (cuDie.isValid()) {
					dfs(cuDie, 0);
				}
				debug_log << "  Total DIEs processed: " << die_count << std::endl;
				
				registry["debug_info"].push_back(cu_info);
				cu_index++;
			}
			
		
		}
		std::ofstream os(out);
		os << registry.dump(2);// JSON ausgeben (inklusive Debug-Infos)

	
	// Ergebnis speichern
	// Nach der Verarbeitung aller CUs
	//llvm::outs() << "Found " << struct_count << " structs/classes\n";
	//llvm::outs() << "Found " << var_count << " variables\n";
	//llvm::outs() << "Found " << fun_count << " functions\n";

	
	
	llvm::outs() << "Successfully wrote to output: " << out << "\n";
	
	// Debug-Log abschließen
	debug_log << "\n===== END OF DEBUG LOG =====" << std::endl;
	debug_log.close();

	
	

	llvm::outs() << "Debug information written to: dwarf_debug.log\n";
	llvm::outs() << "JSON output written to: " << out << "\n";
	return 0;
}
