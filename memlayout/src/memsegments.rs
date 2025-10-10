use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySegment {
    pub start: u64,
    pub size: u64,
    pub name: String,
    pub segment_type: SegmentType,
    pub signed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegmentType {
    GlobalVariable,
    StackFrame,
    HeapAllocation,
    StaticData,
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Visualization {
    pub segments: Vec<VisualizationSegment>,
    pub total_size: usize,
}

impl Visualization {
    pub fn render_html(&self) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n<title>Memory Visualization</title>\n");
        html.push_str("<style>.segment { padding: 4px; color: #fff; }</style>\n");
        html.push_str("</head>\n<body>\n");
        html.push_str("<h1>Memory Visualization</h1>\n");
        html.push_str(&format!("<p>Total Size: {} bytes</p>\n", self.total_size));
        html.push_str("<table border=\"1\" cellspacing=\"0\" cellpadding=\"4\">\n");
        html.push_str("<tr><th>Name</th><th>Address</th><th>Size</th><th>Type</th></tr>\n");
        for seg in &self.segments {
            html.push_str(&format!(
                "<tr class='segment' style='background-color:{}'><td>{}</td><td>0x{:X}</td><td>{} bytes</td><td>{}</td></tr>\n",
                seg.color, seg.name, seg.address, seg.size, seg.segment_type
            ));
        }
        html.push_str("</table>\n</body>\n</html>\n");
        html
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationSegment {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub segment_type: String,
    pub color: String,
}
