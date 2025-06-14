use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Modifier, Style, Stylize, palette::tailwind::CYAN},
    text::Line,
    widgets::{
        Block, BorderType, Clear, HighlightSpacing, List, ListItem, Paragraph, StatefulWidget,
        Widget,
    },
};

use crate::{
    app::{App, AppState, OcrEntry, OcrType, SummaryEntry},
    database,
};

const SELECTED_STYLE: Style = Style::new().bg(CYAN.c600).add_modifier(Modifier::BOLD);
const FOOTER_STYLE: Style = Style::new().fg(CYAN.c600);

impl Widget for &mut App<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [main_area, footer_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

        match self.current_state {
            AppState::Blacklist => {
                self.render_home(main_area, buf);
                self.render_ocr(main_area, buf);
                self.render_edit(main_area, buf, "Add to blacklist".to_string());
            }
            AppState::Category => {
                self.render_convert(main_area, buf);
                self.render_category(main_area, buf);
            }
            AppState::ConvertBon => {
                self.render_convert(main_area, buf);
            }
            AppState::EditBonPrice => {
                self.render_convert(main_area, buf);
                self.render_edit(main_area, buf, "Edit bon price".to_string());
            }
            AppState::EditCategory => {
                self.render_convert(main_area, buf);
                self.render_edit(main_area, buf, "Edit category".to_string());
            }
            AppState::EditName => {
                self.render_convert(main_area, buf);
                self.render_edit(main_area, buf, "Edit name".to_string());
            }
            AppState::EditPrice => {
                self.render_convert(main_area, buf);
                self.render_edit(main_area, buf, "Edit price".to_string());
            }
            AppState::Home => {
                self.render_home(main_area, buf);
            }
            AppState::Import => {
                self.render_home(main_area, buf);
                self.render_import(main_area, buf);
            }
            AppState::OCR => {
                self.render_home(main_area, buf);
                self.render_ocr(main_area, buf);
            }
        }

        self.render_footer(footer_area, buf);
    }
}

impl App<'_> {
    fn render_category(&mut self, area: Rect, buf: &mut Buffer) {
        let popup_area = popup_area(area, 50, 50);
        let categories_block = Block::bordered()
            .title("Categories")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let categories: Vec<ListItem> = self
            .category_list
            .items
            .iter()
            .map(ListItem::from)
            .collect();

        let categories_list = List::new(categories)
            .block(categories_block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);

        Widget::render(Clear, popup_area, buf);
        StatefulWidget::render(
            categories_list,
            popup_area,
            buf,
            &mut self.category_list.state,
        );
    }

    fn render_convert(&mut self, area: Rect, buf: &mut Buffer) {
        let [items_area, details_area] =
            Layout::horizontal([Constraint::Fill(2), Constraint::Fill(1)]).areas(area);

        let [details_area, summary_area] =
            Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]).areas(details_area);

        // items
        let items_block = Block::bordered()
            .title("Items")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let items: Vec<ListItem> = self.new_bon_list.items.iter().map(ListItem::from).collect();

        let items_list = List::new(items)
            .block(items_block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(items_list, items_area, buf, &mut self.new_bon_list.state);

        // details
        let details_block = Block::bordered()
            .title("Details")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let details_line = if let Some(i) = self.new_bon_list.state.selected() {
            let entry = &self.new_bon_list.items[i];
            format!(
                "product: {}\nprice: {} €\ncategory: {}",
                entry.product, entry.price, entry.category
            )
        } else {
            "".to_string()
        };

        let details = Paragraph::new(details_line).block(details_block);

        Widget::render(details, details_area, buf);

        // summary
        let summary_block = Block::bordered()
            .title("Summary")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let summary_line = format!(
            "price (OCR): {} €\nprice (calculated): {:.2} €\ndate: {}",
            self.new_bon_list.price_ocr, self.new_bon_list.price_calc, self.new_bon_list.date
        );

        let summary = Paragraph::new(summary_line).block(summary_block);

        Widget::render(summary, summary_area, buf);
    }

    fn render_edit(&mut self, area: Rect, buf: &mut Buffer, msg: String) {
        let popup_area = popup_area(area, 30, 50);
        let vertical = Layout::vertical([Constraint::Length(3)]).flex(Flex::Center);
        let [edit_area] = vertical.areas(popup_area);

        let block = Block::bordered()
            .title(msg)
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        self.edit_field.set_block(block);

        Widget::render(Clear, edit_area, buf);
        Widget::render(&self.edit_field, edit_area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let text = match self.current_state {
            AppState::Category => "Select: Enter | Close: Esc | Quit: q",
            AppState::ConvertBon => {
                "Edit Category: c | Edit Name: n | Edit Price: p | Delete Entry: x | Edit Bon Price: o | Close: Esc | Quit: q"
            }
            AppState::Home => "Next: j | Previous: k | Import: i | Hide: h | Quit: q",
            AppState::Import => "Next: j | Previous: k | Process: Enter | Close: Esc | Quit: q",
            AppState::OCR => {
                "Blacklist Entry: b  | Delete Entry: x | Import Bon: Enter | Mark Date: d | Mark Sum: s | Close: Esc | Quit: q"
            }
            // use the default for the editing windows
            _ => "Add: Enter | Close: Esc",
        };
        Paragraph::new(text).style(FOOTER_STYLE).render(area, buf);
    }

    fn render_home(&mut self, area: Rect, buf: &mut Buffer) {
        let [bons_area, details_area] =
            Layout::horizontal([Constraint::Fill(2), Constraint::Fill(1)]).areas(area);

        let [details_area, summary_area] =
            Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]).areas(details_area);

        // bons
        let bons_block = Block::bordered()
            .title("Bons")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let bons: Vec<ListItem> = self.bon_list.items.iter().map(ListItem::from).collect();

        let bons_list = List::new(bons)
            .block(bons_block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(bons_list, bons_area, buf, &mut self.bon_list.state);

        // details
        let details_block = Block::bordered()
            .title("Details")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let details: Vec<ListItem> = if let Some(i) = self.bon_list.state.selected() {
            self.bon_list.items[i]
                .entries
                .iter()
                .map(ListItem::from)
                .collect()
        } else {
            Vec::new()
        };

        let details_list = List::new(details).block(details_block);

        Widget::render(details_list, details_area, buf);

        // summary
        let summary_block = Block::bordered()
            .title("Summary")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let summary: Vec<ListItem> = self.bon_summary.iter().map(ListItem::from).collect();

        let summary_list = List::new(summary).block(summary_block);

        Widget::render(summary_list, summary_area, buf);
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

    fn render_ocr(&mut self, area: Rect, buf: &mut Buffer) {
        let ocr_area = popup_area(area, 80, 80);

        let block = Block::bordered()
            .title("OCR")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let items: Vec<ListItem> = self.ocr_list.items.iter().map(ListItem::from).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);

        Widget::render(Clear, ocr_area, buf);
        StatefulWidget::render(list, ocr_area, buf, &mut self.ocr_list.state);
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

impl From<&database::Category> for ListItem<'_> {
    fn from(value: &database::Category) -> Self {
        let line = Line::from(value.category.to_string());
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

impl From<&OcrEntry> for ListItem<'_> {
    fn from(value: &OcrEntry) -> Self {
        let prefix = match value.ocr_type {
            OcrType::Date => "D: ",
            OcrType::Entry => "",
            OcrType::Sum => "S: ",
        };
        let line = Line::from(format!("{}{}", prefix, value.name));
        ListItem::new(line)
    }
}

impl From<&SummaryEntry> for ListItem<'_> {
    fn from(value: &SummaryEntry) -> Self {
        let line = if value.category != "total" {
            Line::from(format!("{} {:.2} €", value.category, value.total))
        } else {
            Line::from(format!("{} {:.2} €", value.category, value.total))
                .add_modifier(Modifier::BOLD)
        };
        ListItem::new(line)
    }
}
