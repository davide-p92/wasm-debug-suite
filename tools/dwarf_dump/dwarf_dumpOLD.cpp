int main(int argc, char** argv) {
    if (argc < 3) {
        errs() << "usage: dwarf_dump <input.wasm> <out.json>\n";
        return 1;
    }
    std::string in = argv[1], out = argv[2];

    // 1. Datei einlesen
    auto bufOrErr = MemoryBuffer::getFile(in);
    if (!bufOrErr) {
        errs() << "Error opening file: " << in << "\n";
        return 1;
    }

    // 2. Objektdatei erstellen
    auto objOrErr = ObjectFile::createObjectFile(bufOrErr->get()->getMemBufferRef());
    if (!objOrErr) {
        errs() << "Error creating object: " << toString(objOrErr.takeError()) << "\n";
        return 1;
    }

    auto& obj = **objOrErr;
    auto ctx = DWARFContext::create(obj);
    json registry = { {"types", json::object()}, {"variables", json::array()} };

    // 3. Rekursive Verarbeitungsfunktion
    std::function<void(DWARFDie)> dfs = [&](DWARFDie d) {
        auto tag = d.getTag();
        if (tag == dwarf::DW_TAG_structure_type || 
            tag == dwarf::DW_TAG_class_type) {
            auto name = getName(d).value_or("<anon>");
            auto size = getUData(d, dwarf::DW_AT_byte_size).value_or(0);
            json fields = json::array();
            
            for (DWARFDie ch = d.getFirstChild(); ch; ch = ch.getSibling()) {
                if(ch.getTag() == dwarf::DW_TAG_member) {
                    auto fname = getName(ch).value_or("<field>");
                    auto foff = getUData(ch, dwarf::DW_AT_data_member_location).value_or(0);
                    std::string ftype = resolveTypeName(ch);
                    fields.push_back({{"name", fname}, {"offset", foff}, {"type", ftype}});
                }
            }
            registry["types"][name] = {{"size", size}, {"fields", fields}};
        }
        else if (tag == dwarf::DW_TAG_variable) {
            auto vname = getName(d).value_or("<var>");
            uint64_t addr = 0;
            
            if (auto loc = d.find(dwarf::DW_AT_location)) {
                if (auto exprLoc = loc->getAsBlock()) {
                    DWARFDataExtractor dataExtr(
                        ArrayRef<uint8_t>(exprLoc->data(), exprLoc->size()),
                        ctx->isLittleEndian(),
                        d.getDwarfUnit()->getAddressByteSize()
                    );
                    DWARFExpression expr(dataExtr, d.getDwarfUnit()->getAddressByteSize());
                    for (const auto &op : expr) {
                        if (op.getCode() == dwarf::DW_OP_addr) {
                            addr = op.getRawOperand(0);
                            break;
                        }
                    }
                }
            }
            
            registry["variables"].push_back({{"name", vname}, {"address", addr}});
        }
        
        // Rekursion für Kinder
        for (DWARFDie ch = d.getFirstChild(); ch; ch = ch.getSibling()) {
            dfs(ch);
        }
    };

    // 4. Compilation Units verarbeiten
    for (auto& cu : ctx->compile_units()) {
        DWARFDie cuDie = cu->getUnitDIE();
        dfs(cuDie); // CU-DIE und alle Kinder verarbeiten
    }

    // 5. Ergebnis speichern
    std::ofstream os(out);
    os << registry.dump(2);
    llvm::outs() << "Successfully wrote output to: " << out << "\n";
    
    return 0;
}
