impl MemoryLayout {
    pub fn generate_visualization(&self) -> Visualization {
        let mut blocks = Vec::new();
        for segment in &self.segments {
            blocks.push(MemoryBlock {
                address_range: (segment.start, segment.start + segment.size),
                label: segment.name.clone(),
                color: match segment.segment_type {
                    SegmentType::GlobalVariable => "#FF6B6B",
                    // ..
                },
            });
        }
        Visualization { blocks }
    }
}
