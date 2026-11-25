//! 按仓库根目录 `theme.json` 移植的 `OneDark Pro Italic Vivid`。
//!
//! 显式颜色优先，其余沿用 Winc 的层次生成规则。映射和无法等价表达的项目见 `THEMES.md`。

use super::palettes::hex;
use super::*;

const BACKGROUND: Color = hex(0x282C34);

/// JSON 的八位颜色按 RRGGBBAA 读取，保留选区、滚动条等覆盖层的透明度。
const fn rgba(value: u32) -> Color {
    hex(value >> 8).alpha((value & 0xFF) as f32 / 255.0)
}

/// `OneDark Pro` 的鲜艳斜体变体，配色来自用户提供的 `theme.json`。
pub static ONE_DARK_PRO: Palette = Palette {
    sidebar_bg: hex(0x21252B),
    tab_fill: hex(0x2C313A),
    tab_edge: hex(0x181A1F),
    tab_divider: hex(0x181A1F),
    group_bg: BACKGROUND,
    input_bg: Some(hex(0x1D1F23)),
    border: hex(0x181A1F),
    popup_border: hex(0x181A1F),
    text_on_accent: hex(0xF8FAFD),
    hover: hex(0x292D35),
    selected: hex(0x2C313A),
    selected_inactive: hex(0x2C313A),
    selected_unfocused: hex(0x2C313A),
    separator: hex(0x181A1F),
    current_line: hex(0x383E4A),
    selection_inactive: rgba(0xFFFFFF10),
    // 终端协议只接收 RGB；光标预先叠到底色上，使协议与编辑区绘制结果一致。
    caret: mix(hex(0x3F5DE6), BACKGROUND, 218.0 / 255.0),
    indent_guide: hex(0x3B4048),
    indent_guide_active: rgba(0xAAB1C026),
    whitespace: hex(0x3B4048),
    search_match: hex(0x314365),
    // 原 JSON 写成 editor.editor.findMatchBackground，按当前搜索结果底色解释。
    search_current: hex(0x42557B),
    match_brace: rgba(0xAAB1C026),
    occurrence: hex(0x484E5B),
    scrollbar: rgba(0x4E566680),
    scrollbar_active: rgba(0x747D9180),
    ruler_occurrence: hex(0xAAB1C0).alpha(0.34),
    ruler_caret: hex(0x9DA5B4),
    drop_target: hex(0x383E4A),
    modified_dot: hex(0xD7DAE0),
    // Winc 的差异底色是不透明色：行内新增按 JSON 的青色半透明标记预合成。
    diff_added: DiffShade {
        word: mix(hex(0x00809B), BACKGROUND, 51.0 / 255.0),
        ..shade(hex(0x89CA78), BACKGROUND, true)
    },
    diff_deleted: shade(hex(0xEF596F), BACKGROUND, true),
    syntax_variable: hex(0xEF596F),
    syntax_type: hex(0xE5C07B),
    syntax_escape: hex(0x57B6C2),
    syntax_tag: hex(0xEF596F),
    syntax_attribute: hex(0xD8985F),
    syntax_heading: hex(0xEF596F),
    syntax_emphasis: hex(0xD55FDE),
    syntax_strong: hex(0xD8985F),
    syntax_link: hex(0xD55FDE),
    syntax_inserted: hex(0x89CA78),
    syntax_deleted: hex(0xEF596F),
    syntax_warning: hex(0xCD9731),
    // Winc 不区分控制关键字与其他关键字、HTML 属性与其他属性，按类别近似。
    syntax_italic: &[
        TokenKind::Comment,
        TokenKind::Keyword,
        TokenKind::Attribute,
        TokenKind::Emphasis,
    ],
    terminal_text: hex(0xC8C8C8),
    ..Palette::build(Tokens {
        dark: true,
        frame: hex(0x21252B),
        card: BACKGROUND,
        popup: hex(0x21252B),
        text: hex(0xD7DAE0),
        text_muted: hex(0x9DA5B4),
        text_dim: hex(0x6B717D),
        accent: hex(0x52ADF2),
        accent_fill: hex(0x4D78CC),
        selection: rgba(0x67769660),
        gutter: [hex(0x495162), hex(0xAAB1C0)],
        error: hex(0xC24038),
        warning: hex(0xCD9731),
        git: [hex(0xE0C285), hex(0x89CA78), hex(0xEF596F), hex(0xF44747)],
        tinted_tab: false,
        syntax: [
            // 文件没有 editor.foreground，正文取通用标点/代码重置色。
            hex(0xAAB1C0),
            hex(0xD55FDE),
            hex(0xE5C07B),
            hex(0x89CA78),
            hex(0xD8985F),
            hex(0x5C6370),
            hex(0x52ADF2),
            hex(0x2BBAC5),
            hex(0xEF596F),
        ],
        // JSON 只指定黑、绿、黄、蓝；其余取同系语法色及较亮的变体补齐。
        ansi: [
            hex(0x2D3139),
            hex(0xEF596F),
            hex(0x98C379),
            hex(0xB4881D),
            hex(0x2E8CCF),
            hex(0xD55FDE),
            hex(0x2BBAC5),
            hex(0xAAB1C0),
            hex(0x7F848E),
            hex(0xF27888),
            hex(0xB3D99C),
            hex(0xE5C07B),
            hex(0x52ADF2),
            hex(0xE48BEC),
            hex(0x57B6C2),
            hex(0xF8FAFD),
        ],
    })
};
