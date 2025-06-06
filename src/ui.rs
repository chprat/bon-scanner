use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Modifier, Style, Stylize, palette::tailwind::CYAN},
    text::Line,
    widgets::{
        Block, BorderType, Clear, HighlightSpacing, List, ListItem, Paragraph, StatefulWidget,
        Widget, Wrap,
    },
};

use crate::{
    app::{App, AppState, SummaryEntry},
    database,
};

const SELECTED_STYLE: Style = Style::new().bg(CYAN.c600).add_modifier(Modifier::BOLD);
const FOOTER_STYLE: Style = Style::new().fg(CYAN.c600);

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [main_area, footer_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

        let [list_area, details_area] =
            Layout::horizontal([Constraint::Fill(2), Constraint::Fill(1)]).areas(main_area);

        let [items_area, summary_area] =
            Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]).areas(details_area);

        self.render_details(items_area, buf);
        self.render_footer(footer_area, buf);
        self.render_list(list_area, buf);
        self.render_summary(summary_area, buf);

        if matches!(self.current_state, AppState::Import) {
            self.render_import(area, buf);
        }

        if matches!(self.current_state, AppState::OCR) {
            self.render_ocr(area, buf);
        }
    }
}

impl App {
    fn render_details(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("Details")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let items: Vec<ListItem> = if let Some(i) = self.bon_list.state.selected() {
            self.bon_list.items[i]
                .entries
                .iter()
                .map(ListItem::from)
                .collect()
        } else {
            Vec::new()
        };

        let list = List::new(items).block(block);

        Widget::render(list, area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let text = match self.current_state {
            AppState::Home => "Next: j | Previous: k | Import: i | Quit: q",
            AppState::Import => "Next: j | Previous: k | Process: Enter | Close: Esc | Quit: q",
            AppState::OCR => "Close: Esc | Quit: q",
        };
        Paragraph::new(text).style(FOOTER_STYLE).render(area, buf);
    }

    fn render_import(&mut self, area: Rect, buf: &mut Buffer) {
        let import_area = popup_area(area, 50, 50);

        let block = Block::bordered()
            .title("Files")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let items: Vec<ListItem> = self
            .import_list
            .items
            .iter()
            .map(|elem| ListItem::from(elem.as_str()))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);

        Widget::render(Clear, import_area, buf);
        StatefulWidget::render(list, import_area, buf, &mut self.import_list.state);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("Bons")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let items: Vec<ListItem> = self.bon_list.items.iter().map(ListItem::from).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.bon_list.state);
    }

    fn render_ocr(&mut self, area: Rect, buf: &mut Buffer) {
        let ocr_area = popup_area(area, 50, 50);

        let block = Block::bordered()
            .title("OCR")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        Widget::render(Clear, ocr_area, buf);
        Paragraph::new(self.ocr_text.join("\n"))
            .wrap(Wrap { trim: true })
            .centered()
            .block(block)
            .render(ocr_area, buf);
    }

    fn render_summary(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("Summary")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let items: Vec<ListItem> = self.bon_summary.iter().map(ListItem::from).collect();

        let list = List::new(items).block(block);

        Widget::render(list, area, buf);
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

impl From<&database::Bon> for ListItem<'_> {
    fn from(value: &database::Bon) -> Self {
        let line = Line::from(format!("{} {} €", value.date, value.price));
        ListItem::new(line)
    }
}

impl From<&database::Entry> for ListItem<'_> {
    fn from(value: &database::Entry) -> Self {
        let line = Line::from(format!(
            "{} {} {} €",
            value.category, value.product, value.price
        ));
        ListItem::new(line)
    }
}

impl From<&SummaryEntry> for ListItem<'_> {
    fn from(value: &SummaryEntry) -> Self {
        let line = if value.category != "total" {
            Line::from(format!("{} {} €", value.category, value.total))
        } else {
            Line::from(format!("{} {} €", value.category, value.total)).add_modifier(Modifier::BOLD)
        };
        ListItem::new(line)
    }
}
