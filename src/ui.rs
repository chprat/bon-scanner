use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::app::App;

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("bon-scanner")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let text = "Press `Esc`, `Ctrl-C` or `q` to stop running.";

        let paragraph = Paragraph::new(text).block(block).centered();

        paragraph.render(area, buf);
    }
}
