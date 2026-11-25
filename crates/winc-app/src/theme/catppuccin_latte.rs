//! Catppuccin Latte：冷白底、淡灰紫层次与紫色强调。
//!
//! 按上游 VS Code 主题 3.18.1 适配；语法色为 Winc 的浅色对比度要求略微加深，见 `THEMES.md`。

use super::palettes::hex;
use super::*;

const BACKGROUND: Color = hex(0xEFF1F5);

/// Catppuccin 的浅色变体，保留界面底色、强调色与完整的语法分类。
pub static CATPPUCCIN_LATTE: Palette = Palette {
    sidebar_bg: hex(0xE6E9EF),
    tab_fill: BACKGROUND,
    tab_edge: hex(0x8839EF),
    tab_divider: hex(0xE6E9EF),
    group_bg: BACKGROUND,
    input_bg: Some(hex(0xCCD0DA)),
    border: hex(0xBCC0CC),
    popup_border: hex(0xBCC0CC),
    text_on_accent: hex(0xDCE0E8),
    hover: hex(0xCCD0DA).alpha(128.0 / 255.0),
    selected: hex(0xCCD0DA),
    selected_inactive: hex(0xCCD0DA),
    selected_unfocused: hex(0xCCD0DA),
    separator: hex(0xBCC0CC),
    current_line: hex(0x4C4F69).alpha(18.0 / 255.0),
    selection_inactive: hex(0x7C7F93).alpha(0.2),
    caret: hex(0xDC8A78),
    indent_guide: hex(0xBCC0CC),
    indent_guide_active: hex(0xACB0BE),
    whitespace: hex(0x7C7F93).alpha(0.4),
    search_match: hex(0xA9DAF0),
    search_current: hex(0xE6ADBD),
    match_brace: hex(0x7C7F93).alpha(26.0 / 255.0),
    occurrence: hex(0x7C7F93).alpha(0.2),
    scrollbar: hex(0xACB0BE).alpha(128.0 / 255.0),
    // Winc 的悬停和拖动共用一色，采用上游更清晰的悬停色。
    scrollbar_active: hex(0x9CA0B0),
    ruler_occurrence: hex(0x7C7F93).alpha(0.4),
    ruler_caret: hex(0xDC8A78),
    drop_target: hex(0x8839EF).alpha(0.2),
    modified_dot: hex(0xDF8E1D),
    // 差异区域使用不透明颜色，将上游半透明的行/词标记预合成到底色上。
    diff_added: DiffShade {
        line: mix(hex(0x40A02B), BACKGROUND, 38.0 / 255.0),
        word: mix(hex(0x40A02B), BACKGROUND, 0.2),
        ..shade(hex(0x40A02B), BACKGROUND, false)
    },
    diff_deleted: DiffShade {
        line: mix(hex(0xD20F39), BACKGROUND, 38.0 / 255.0),
        word: mix(hex(0xD20F39), BACKGROUND, 0.2),
        ..shade(hex(0xD20F39), BACKGROUND, false)
    },
    syntax_variable: hex(0x4C4F69),
    syntax_type: hex(0x7F5110),
    syntax_escape: hex(0x864374),
    syntax_tag: hex(0x1852C6),
    syntax_attribute: hex(0x7F5110),
    syntax_heading: hex(0xB60D31),
    syntax_emphasis: hex(0xB60D31),
    syntax_strong: hex(0xB60D31),
    syntax_link: hex(0x1852C6),
    syntax_inserted: hex(0x29681B),
    syntax_deleted: hex(0xB60D31),
    syntax_warning: hex(0x9C3D06),
    // 逐语言 scope 无法等价映射，斜体按上游的通用语法类别应用。
    syntax_italic: &[
        TokenKind::Comment,
        TokenKind::Function,
        TokenKind::Type,
        TokenKind::Emphasis,
    ],
    terminal_text: hex(0x4C4F69),
    ..Palette::build(Tokens {
        dark: false,
        frame: hex(0xDCE0E8),
        card: BACKGROUND,
        popup: hex(0xE6E9EF),
        text: hex(0x4C4F69),
        text_muted: hex(0x5C5F77),
        text_dim: hex(0x6C6F85),
        accent: hex(0x8839EF),
        accent_fill: hex(0x8839EF),
        selection: hex(0x7C7F93).alpha(77.0 / 255.0),
        gutter: [hex(0x878A9C), hex(0x8839EF)],
        error: hex(0xD20F39),
        warning: hex(0xFE640B),
        git: [hex(0xDF8E1D), hex(0x40A02B), hex(0xD20F39), hex(0x8839EF)],
        tinted_tab: false,
        // 保持原色相，正文语法至少 6:1，注释至少 4.5:1。
        syntax: [
            hex(0x4C4F69),
            hex(0x7531CE),
            hex(0xB60D31),
            hex(0x29681B),
            hex(0x9C3D06),
            hex(0x6A6D7E),
            hex(0x1852C6),
            hex(0x9C3D06),
            hex(0x1852C6),
        ],
        // 上游 ANSI 色中偏浅的颜色加深到 3:1；亮白保留供终端程序铺底。
        ansi: [
            hex(0x5C5F77),
            hex(0xD20F39),
            hex(0x3F9E2A),
            hex(0xC27B19),
            hex(0x1E66F5),
            hex(0xCC67B1),
            hex(0x179299),
            hex(0x878B96),
            hex(0x6C6F85),
            hex(0xDE293E),
            hex(0x419E37),
            hex(0xBC7E23),
            hex(0x456EFF),
            hex(0xCA6AAC),
            hex(0x2B99A1),
            hex(0xBCC0CC),
        ],
    })
};
