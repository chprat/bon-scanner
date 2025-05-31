use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, BorderType, List, ListItem, Paragraph, StatefulWidget, Widget},
};

use crate::{app::App, database};

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [main_area, footer_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

        let [list_area, details_area] =
            Layout::horizontal([Constraint::Fill(2), Constraint::Fill(1)]).areas(main_area);

        App::render_footer(footer_area, buf);
        self.render_details(details_area, buf);
        self.render_list(list_area, buf);
    }
}

impl App {
    fn render_details(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("Details")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);
        let text = "Details of the selected bon";
        let paragraph = Paragraph::new(text).block(block).centered();
        paragraph.render(area, buf);
    }

    fn render_footer(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Quit: q").render(area, buf);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("Bons")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let items: Vec<ListItem> = self.bon_list.items.iter().map(ListItem::from).collect();

        let list = List::new(items).block(block);

        StatefulWidget::render(list, area, buf, &mut self.bon_list.state);
    }
}

impl From<&database::Bon> for ListItem<'_> {
    fn from(value: &database::Bon) -> Self {
        let line = Line::from(format!("{} {} €", value.date, value.price));
        ListItem::new(line)
    }
}
