use cursive::With;
use cursive::style::{
    BaseColor, BorderStyle, Color, ColorStyle, Effect, Palette, PaletteColor, PaletteStyle, Style,
};
use cursive::theme::Theme;

pub fn theme() -> Theme {
    Theme {
        shadow: false,
        borders: BorderStyle::Simple,
        palette: Palette::retro().with(|palette| {
            palette[PaletteColor::Background] = Color::TerminalDefault;
            palette[PaletteColor::View] = BaseColor::Black.dark();
            palette[PaletteColor::Primary] = BaseColor::White.light();
            palette[PaletteColor::Secondary] = BaseColor::Blue.light();
            palette[PaletteColor::Tertiary] = BaseColor::Yellow.light();
            palette[PaletteColor::Highlight] = BaseColor::Blue.dark();
            palette[PaletteColor::HighlightText] = BaseColor::White.light();
            palette[PaletteStyle::EditableText] = Style::from(ColorStyle::new(
                BaseColor::Black.dark(),
                BaseColor::White.dark(),
            ));
            palette[PaletteStyle::EditableTextCursor] = Style::from(ColorStyle::new(
                BaseColor::White.dark(),
                BaseColor::Blue.dark(),
            ))
            .combine(Effect::Bold);
            palette[PaletteStyle::TitlePrimary] =
                Style::from(BaseColor::Yellow.light()).combine(Effect::Bold);
            palette[PaletteStyle::HighlightInactive] = Style::from(ColorStyle::new(
                BaseColor::White.light(),
                BaseColor::Black.light(),
            ));
        }),
    }
}
