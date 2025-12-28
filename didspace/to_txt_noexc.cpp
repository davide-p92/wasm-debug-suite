#include <iostream>
#include <fstream>
#include <filesystem>
#include <string>
#include <vector>
#include <map>
#include <algorithm>
#include <cctype>

namespace fs = std::filesystem;

// Map für Kommentarstile
std::map<std::string, std::pair<std::string, std::string>> comment_styles = {
    {"c", {"//", ""}},       {"h", {"//", ""}},       {"cpp", {"//", ""}},
    {"hpp", {"//", ""}},     {"cs", {"//", ""}},      {"js", {"//", ""}},
    {"ts", {"//", ""}},      {"jsx", {"//", ""}},     {"m", {"//", ""}},
    {"rs", {"//", ""}},      {"py", {"#", ""}},       {"rb", {"#", ""}},
    {"pl", {"#", ""}},       {"sh", {"#", ""}},       {"html", {"<!--", "-->"}},
    {"css", {"/*", "*/"}},   {"php", {"//", ""}},     {"swift", {"//", ""}},
    {"kt", {"//", ""}},      {"java", {"//", ""}},    {"go", {"//", ""}},
    {"eml", {"//", ""}},
};

// Hilfsfunktion: Erweiterung sicher extrahieren
std::string get_extension(const fs::path& path) {
    if (!path.has_extension()) return "";
    std::string ext = path.extension().string();
    if (!ext.empty() && ext[0] == '.') {
        ext = ext.substr(1);
    }
    std::transform(ext.begin(), ext.end(), ext.begin(),
        [](unsigned char c){ return std::tolower(c); });
    return ext;
}

// Hauptfunktion zum Verarbeiten der Dateien
void process_file(const fs::path& file_path, const fs::path& base_dir, std::ofstream& out_file) {
    fs::path rel_path = fs::relative(file_path, base_dir);
    std::string ext = get_extension(rel_path);
    
    auto comment_style = comment_styles.find(ext);
    if (comment_style == comment_styles.end()) {
        comment_style = comment_styles.find("c");
    }
    
    if (comment_style->second.second.empty()) {
        out_file << comment_style->second.first << " " << rel_path.string() << "\n";
    } else {
        out_file << comment_style->second.first << " " << rel_path.string() << " " << comment_style->second.second << "\n";
    }
    
    std::ifstream in_file(file_path, std::ios::binary);
    if (in_file) {
        out_file << in_file.rdbuf();
        out_file << "\n\n";
    } else {
        std::cerr << "Fehler beim Lesen: " << file_path << "\n";
    }
}

// Prüft ob ein Pfad in der Ausschlussliste enthalten ist
bool is_excluded(const fs::path& path, const std::vector<fs::path>& exclude_dirs) {
    for (const auto& excl_dir : exclude_dirs) {
        // Prüft ob der Pfad mit einem ausgeschlossenen Verzeichnis beginnt
        if (path.string().find(excl_dir.string()) == 0) {
            return true;
        }
    }
    return false;
}

void print_help(const char* program_name) {
    std::cout << "Verwendung: " << program_name << " [Optionen]\n\n"
              << "Optionen:\n"
              << "  --fext <erweiterungen>   Kommagetrennte Liste von Dateierweiterungen (Standard: c,h,cpp,hpp)\n"
              << "  --startdir <verzeichnis> Startverzeichnis (Standard: aktuelles Verzeichnis)\n"
              << "  --exclude_dir <verzeichnisse> Kommagetrennte Liste von auszuschließenden Verzeichnissen (inkl. Unterverzeichnissen)\n"
              << "  --fout <dateiname>      Ausgabedateiname (Standard: output.txt)\n"
              << "  --help                  Diese Hilfeseite anzeigen\n\n"
              << "Unterstützte Dateitypen:\n";
    
    for (const auto& [ext, _] : comment_styles) {
        std::cout << "  ." << ext << "\n";
    }
}

int main(int argc, char* argv[]) {
    // Default-Werte
    std::vector<std::string> extensions = {"c", "h", "cpp", "hpp"};
    std::vector<fs::path> exclude_dirs;
    fs::path start_dir = ".";
    std::string output_file = "output.txt";
    
    // Argumente verarbeiten
    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg == "--fext" && i + 1 < argc) {
            extensions.clear();
            std::string exts = argv[++i];
            size_t pos = 0;
            while ((pos = exts.find(',')) != std::string::npos) {
                extensions.push_back(exts.substr(0, pos));
                exts.erase(0, pos + 1);
            }
            extensions.push_back(exts);
        } else if (arg == "--startdir" && i + 1 < argc) {
            start_dir = fs::absolute(argv[++i]);
        } else if (arg == "--exclude_dir" && i + 1 < argc) {
            std::string dirs = argv[++i];
            size_t pos = 0;
            while ((pos = dirs.find(',')) != std::string::npos) {
                exclude_dirs.push_back(fs::absolute(dirs.substr(0, pos)));
                dirs.erase(0, pos + 1);
            }
            exclude_dirs.push_back(fs::absolute(dirs));
        } else if (arg == "--fout" && i + 1 < argc) {
            output_file = argv[++i];
        } else if (arg == "--help") {
            print_help(argv[0]);
            return 0;
        } else {
            std::cerr << "Unbekannte Option: " << arg << "\n";
            print_help(argv[0]);
            return 1;
        }
    }
    
    // Erweiterungen normalisieren
    for (auto& ext : extensions) {
        std::transform(ext.begin(), ext.end(), ext.begin(),
            [](unsigned char c){ return std::tolower(c); });
    }
    
    // Startverzeichnis absolut machen
    start_dir = fs::absolute(start_dir);
    
    // Ausgabedatei öffnen
    std::ofstream out_stream(output_file, std::ios::binary);
    if (!out_stream) {
        std::cerr << "Fehler beim Öffnen der Ausgabedatei: " << output_file << "\n";
        return 1;
    }
    
//    try {
        // Korrigierte Iterator-Behandlung
        auto iter = fs::recursive_directory_iterator(start_dir);
        for (auto it = begin(iter); it != end(iter); ++it) {
            const auto& entry = *it;
            
            // Prüfen ob der Pfad ausgeschlossen werden soll
            if (is_excluded(entry.path(), exclude_dirs)) {
                // Rekursion in diesem Verzeichnisbaum deaktivieren
                it.disable_recursion_pending();
                continue;
            }
            
            // Nur reguläre Dateien verarbeiten
            if (entry.is_regular_file()) {
                std::string ext = get_extension(entry.path());
                
                if (std::find(extensions.begin(), extensions.end(), ext) != extensions.end()) {
                    process_file(entry.path(), start_dir, out_stream);
                }
            }
        }
  /*  } catch (const fs::filesystem_error& e) {
        std::cerr << "Dateisystemfehler: " << e.what() << "\n";
        return 1;
    }*/
    
    std::cout << "Datei " << output_file << " erfolgreich erstellt\n";
    return 0;
}
