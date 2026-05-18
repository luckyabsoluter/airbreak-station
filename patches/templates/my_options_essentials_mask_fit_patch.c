#include <stdint.h>

/*
 * Entry hook target:
 *   0x0806177E: bl 0x08064E8C  (Essentials=Plus rendered My Options)
 *   0x0806194E: bl 0x08064E8C  (Essentials=On compact My Options)
 *
 * This payload is used from the compact Essentials=On My Options path and from
 * the rendered Essentials=Plus My Options object. The hook preserves the stock append,
 * then appends independent AirBreak-owned rows so they stay at the bottom:
 *   - Block Breaker navigation row
 *   - Custom About navigation row
 *   - Clinical Mode navigation row
 *
 * Custom About deliberately does not reuse the stock About label/page or patch
 * About-page strings. It routes to an AirBreak-owned page slot whose tail append
 * is hooked below so the visible body contains only the custom detail text.
 */

#define MENU_COUNT_OFF 0x0Cu
#define MENU_ITEMS_OFF 0x08u
#define MENU_CAPACITY_OFF 0x1Cu
#define NAV_ROW_ACTION_OFF 0x10u
#define NAV_ACTION_ALLOC_SIZE 0x08u
#define NAV_ROW_ALLOC_SIZE 0x14u
#define SIMPLE_TEXT_ROW_ALLOC_SIZE 0x0Cu
#define CUSTOM_ABOUT_LABEL_ID 0x0E2u
#define CUSTOM_ABOUT_DETAIL_ID 0x0E3u
#define CUSTOM_PAGE_TITLE_ID 0x0E8u
#define AIRBREAK_CUSTOM_PAGE_INDEX 0x0Eu
#define BLOCK_BREAKER_LABEL_ID 0x0E7u
#define BLOCK_BREAKER_STATUS_ID 0x0E9u
#define BLOCK_BREAKER_ROW0_ID 0x146u
#define BLOCK_BREAKER_ROW1_ID 0x147u
#define BLOCK_BREAKER_ROW2_ID 0x16Cu
#define BLOCK_BREAKER_LEFT_LABEL_ID 0x16Fu
#define BLOCK_BREAKER_RIGHT_LABEL_ID 0x1B2u
#define BLOCK_BREAKER_FIRE_LABEL_ID 0x1B3u
#define BLOCK_BREAKER_PAGE_INDEX AIRBREAK_CUSTOM_PAGE_INDEX
#define BLOCK_BREAKER_ALT_PAGE_INDEX 0x0Du
#define RETURN_PAGE_PLUS_MY_OPTIONS 0x01u
#define RETURN_PAGE_COMPACT_MY_OPTIONS 0x02u
#define CLINICAL_MODE_LABEL_ID 0x03Au
#define CLINICAL_MODE_PAGE_INDEX 0x06u
#define CLINICAL_MODE_PAGE_ROW 0x0037u
#define NAV_ROW_STYLE 0x29u
#define EVENT_PAGE_INDEX 0x01BCu
#define EVENT_PAGE_ROW 0x01BDu
#define EVENT_SELECTED_ROW 0x01BFu
#define AIRBREAK_CUSTOM_ABOUT_RETURN_STATE_ADDR 0x2001FFF0u
#define AIRBREAK_CUSTOM_PAGE_MENU_ADDR 0x2001FC9Cu
#define AIRBREAK_BLOCK_BREAKER_RETURN_STATE_ADDR 0x2001FCA0u
#define AIRBREAK_BLOCK_BREAKER_STATE_ADDR 0x2001FCA4u
#define AIRBREAK_BLOCK_BREAKER_DRAW_PENDING_ADDR 0x2001FCA8u
#define AIRBREAK_BLOCK_BREAKER_RENDER_COUNT_ADDR 0x2001FCACu
#define AIRBREAK_BLOCK_BREAKER_BUTTON_STATE_ADDR 0x2001FCB0u
#define AIRBREAK_BLOCK_BREAKER_LAST_ROW_ADDR 0x2001FCB4u
#define AIRBREAK_BLOCK_BREAKER_TICK_COUNTER_ADDR 0x2001FCB8u
#define AIRBREAK_BLOCK_BREAKER_PREV_STATE_ADDR 0x2001FCBCu
#define AIRBREAK_BLOCK_BREAKER_BLOCKS_ADDR 0x2001FCC0u
#define AIRBREAK_CUSTOM_PAGE_TITLE_TEXT_ADDR 0x2001FCE0u
#define AIRBREAK_BLOCK_BREAKER_STATUS_TEXT_ADDR 0x2001FD00u
#define AIRBREAK_BLOCK_BREAKER_ROW0_TEXT_ADDR 0x2001FD20u
#define AIRBREAK_BLOCK_BREAKER_ROW1_TEXT_ADDR 0x2001FD40u
#define AIRBREAK_BLOCK_BREAKER_ROW2_TEXT_ADDR 0x2001FD60u
#define AIRBREAK_ROTARY_PROVIDER_ADDR 0x200174E4u
#define AIRBREAK_RETURN_STATE_MAGIC 0xA5000000u
#define AIRBREAK_RETURN_STATE_MAGIC_MASK 0xFF000000u
#define AIRBREAK_BLOCK_BREAKER_STATE_MAGIC 0xB4000000u
#define AIRBREAK_BLOCK_BREAKER_STATE_MAGIC_MASK 0xFF000000u
#define AIRBREAK_BLOCK_BREAKER_ALT_PAGE_BIT 0x00400000u
#define AIRBREAK_BLOCK_BREAKER_DY_BIT 0x00800000u
#define HOOK_RETURN_PLUS_MY_OPTIONS 0x08061782u
#define HOOK_RETURN_COMPACT_MY_OPTIONS 0x08061952u
#define ADDR_CUSTOM_ABOUT_DETAIL_PTR_SLOT 0x080207A8u

#define BLOCK_BREAKER_CMD_TICK 0u
#define BLOCK_BREAKER_CMD_LEFT 1u
#define BLOCK_BREAKER_CMD_RIGHT 2u
#define BLOCK_BREAKER_CMD_FIRE 3u
#define CUSTOM_PAGE_VISIBLE_CUSTOM_COUNT 2u
#define CUSTOM_PAGE_VISIBLE_BLOCK_BREAKER_COUNT 0u
#define BLOCK_BREAKER_TICK_DIVISOR 0x20000u
#define BLOCK_BREAKER_INITIAL_BLOCKS 0x0003FFFFu
#define BLOCK_BREAKER_ENCODER_ACC_NEUTRAL 4u
#define BLOCK_BREAKER_ENCODER_ACC_MAX 8u
#define ROTARY_COUNT_OFF 4u
#define ROTARY_DIRECTION_OFF 20u
#define ROTARY_CHANGED_OFF 28u
#define ROTARY_PENDING_OFF 29u

#define LCD_CMD_ADDR 0x64000000u
#define LCD_DATA_ADDR 0x64000002u
#define LCD_WIDTH 240u
#define LCD_HEIGHT 320u
#define LCD_COLOR_BLACK 0x0000u
#define LCD_COLOR_DIM 0x1082u
#define LCD_COLOR_WHITE 0xFFFFu
#define LCD_COLOR_GREEN 0x07E0u
#define LCD_COLOR_RED 0x001Fu
#define LCD_COLOR_BLUE 0xF800u
#define LCD_COLOR_YELLOW 0x07FFu
#define GPIOF_IDR_ADDR 0x40021410u
#define GPIOF_ENCODER_A_MASK 0x00000400u
#define GPIOF_ENCODER_B_MASK 0x00000800u
#define GPIOG_IDR_ADDR 0x40021810u
#define GPIOG_HOME_BUTTON_MASK 0x00000080u
#define GPIOG_ENCODER_BUTTON_MASK 0x00000800u

#define ADDR_ALLOC 0x08063D5Cu
#define ADDR_MENU_APPEND 0x08064E8Cu
#define ADDR_NAV_ACTION_CTOR 0x08063C28u
#define ADDR_NAV_ROW_CTOR 0x08065604u
#define ADDR_SIMPLE_TEXT_ROW_CTOR 0x08065FD4u
#define ADDR_EVENT_SET 0x08066E7Eu
#define ADDR_EVENT_GET 0x08066FC6u
#define ADDR_POST_RENDER_WAIT 0x0808EAB4u
#define ADDR_NAV_ACTION_VFUNC_0 0x08063C33u
#define ADDR_NAV_ACTION_VFUNC_8 0x08063C23u

typedef uint32_t (*fn_alloc_t)(uint32_t size);
typedef uint32_t (*fn_menu_append_t)(uint32_t menu_obj, uint32_t item_obj, uint32_t p3, uint32_t p4);
typedef uint32_t (*fn_nav_action_ctor_t)(uint32_t action_obj, uint32_t page_index, uint32_t page_row);
typedef uint32_t (*fn_nav_row_ctor_t)(uint32_t item_obj, uint32_t label_id, uint32_t flags, uint32_t action_obj, uint32_t style);
typedef uint32_t (*fn_simple_text_row_ctor_t)(uint32_t item_obj, uint32_t label_id);
typedef uint32_t (*fn_event_get_t)(uint32_t event_id);
typedef uint32_t (*fn_event_set_t)(uint32_t event_id, uint32_t value);
typedef uint32_t (*fn_action_vfunc_t)(uint32_t action_obj);
typedef uint32_t (*fn_wait_predicate_t)(uint32_t wait_base, uint32_t wait_ticks);

static uint32_t normalize_my_options_return_page(uint32_t return_page);
static void block_breaker_copy_text(uint32_t dst_addr, const char *src);
static void copy_custom_about_detail_to_status(void);
static uint32_t block_breaker_valid_state(uint32_t state);
static void block_breaker_store_state(uint32_t state);
static void custom_page_set_visible_count(uint32_t count);
static uint32_t block_breaker_page_index_for_state(uint32_t state);
static void render_airbreak_page_text_for_state(uint32_t state);
static void block_breaker_draw_full_frame(uint32_t state);
static void block_breaker_request_frame_draw(void);
static uint32_t block_breaker_apply_command(uint32_t state, uint32_t command);
static void block_breaker_clear_rotary_provider(void);
static uint32_t patch_custom_about_back_exec(uint32_t action_obj);

static void store_custom_about_return_state(uint32_t return_page, uint32_t return_row) {
    volatile uint32_t *state = (volatile uint32_t *)AIRBREAK_CUSTOM_ABOUT_RETURN_STATE_ADDR;

    *state = AIRBREAK_RETURN_STATE_MAGIC |
        ((return_page & 0xFFu) << 16) |
        (return_row & 0xFFFFu);
}

static uint32_t load_custom_about_return_page(uint32_t packed_state) {
    if ((packed_state & AIRBREAK_RETURN_STATE_MAGIC_MASK) != AIRBREAK_RETURN_STATE_MAGIC) {
        return 0u;
    }
    return normalize_my_options_return_page((packed_state >> 16) & 0xFFu);
}

static uint32_t load_custom_about_return_row(uint32_t packed_state) {
    if ((packed_state & AIRBREAK_RETURN_STATE_MAGIC_MASK) != AIRBREAK_RETURN_STATE_MAGIC) {
        return 0u;
    }
    return packed_state & 0xFFFFu;
}

__attribute__((used, noinline))
static uint32_t patch_custom_about_entry_exec(uint32_t action_obj) {
    fn_event_get_t event_get_fn = (fn_event_get_t)(ADDR_EVENT_GET | 1u);
    fn_event_set_t event_set_fn = (fn_event_set_t)(ADDR_EVENT_SET | 1u);
    uint32_t return_page = 0u;
    uint32_t return_row = event_get_fn(EVENT_SELECTED_ROW);

    if (action_obj != 0u) {
        return_page = normalize_my_options_return_page(*(uint8_t *)(action_obj + 4u));
    }
    if (return_page == 0u) {
        return_page = normalize_my_options_return_page(event_get_fn(EVENT_PAGE_INDEX));
    }
    if (return_page == 0u) {
        return_page = RETURN_PAGE_COMPACT_MY_OPTIONS;
    }

    store_custom_about_return_state(return_page, return_row);
    block_breaker_store_state(0u);
    block_breaker_copy_text(AIRBREAK_CUSTOM_PAGE_TITLE_TEXT_ADDR, "Custom About");
    copy_custom_about_detail_to_status();
    custom_page_set_visible_count(CUSTOM_PAGE_VISIBLE_CUSTOM_COUNT);
    event_set_fn(EVENT_PAGE_INDEX, AIRBREAK_CUSTOM_PAGE_INDEX);
    event_set_fn(EVENT_PAGE_ROW, 0u);
    return 0u;
}

static void block_breaker_clear_runtime_state(void) {
    block_breaker_store_state(0u);
    block_breaker_clear_rotary_provider();
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_DRAW_PENDING_ADDR = 0u;
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_RENDER_COUNT_ADDR = 0u;
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_BUTTON_STATE_ADDR = 0u;
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_LAST_ROW_ADDR = 0u;
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_TICK_COUNTER_ADDR = 0u;
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_PREV_STATE_ADDR = 0u;
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_BLOCKS_ADDR = 0u;
}

static void block_breaker_release_rotary_provider_to_ui(void) {
    *(volatile uint8_t *)(AIRBREAK_ROTARY_PROVIDER_ADDR + ROTARY_CHANGED_OFF) = 1u;
}

__attribute__((used, noinline))
static uint32_t patch_custom_about_back_exec(uint32_t action_obj) {
    fn_event_set_t event_set_fn = (fn_event_set_t)(ADDR_EVENT_SET | 1u);
    uint32_t state = *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_STATE_ADDR;
    uint32_t block_active = block_breaker_valid_state(state);
    uint32_t packed_state = *(volatile uint32_t *)(
        block_active ?
            AIRBREAK_BLOCK_BREAKER_RETURN_STATE_ADDR :
            AIRBREAK_CUSTOM_ABOUT_RETURN_STATE_ADDR
    );
    uint32_t return_page = load_custom_about_return_page(packed_state);
    uint32_t return_row = load_custom_about_return_row(packed_state);

    if (block_active && action_obj != 0u) {
        event_set_fn(EVENT_PAGE_INDEX, block_breaker_page_index_for_state(state));
        block_breaker_request_frame_draw();
        return 0u;
    }

    if (return_page == 0u) {
        return_page = RETURN_PAGE_COMPACT_MY_OPTIONS;
        return_row = 0u;
    }

    if (block_active) {
        custom_page_set_visible_count(CUSTOM_PAGE_VISIBLE_CUSTOM_COUNT);
        block_breaker_clear_runtime_state();
        block_breaker_release_rotary_provider_to_ui();
    }

    event_set_fn(EVENT_PAGE_INDEX, return_page);
    event_set_fn(EVENT_PAGE_ROW, return_row);
    return 0u;
}

static uint32_t block_breaker_pack_state(
    uint32_t paddle,
    uint32_t ball_x,
    uint32_t ball_y,
    uint32_t dx,
    uint32_t dy,
    uint32_t active
) {
    return AIRBREAK_BLOCK_BREAKER_STATE_MAGIC |
        (paddle & 0x07u) |
        ((ball_x & 0x0Fu) << 3) |
        ((ball_y & 0x0Fu) << 7) |
        ((dx & 0x01u) << 11) |
        ((active & 0x01u) << 12) |
        ((dy & 0x01u) << 23);
}

static uint32_t block_breaker_paddle_ball_x(uint32_t paddle) {
    if (paddle > 4u) {
        paddle = 4u;
    }
    return (paddle * 3u) + 2u;
}

static uint32_t block_breaker_initial_state(void) {
    return block_breaker_pack_state(
        2u,
        block_breaker_paddle_ball_x(2u),
        15u,
        1u,
        0u,
        0u
    );
}

static uint32_t block_breaker_valid_state(uint32_t state) {
    return (state & AIRBREAK_BLOCK_BREAKER_STATE_MAGIC_MASK) ==
        AIRBREAK_BLOCK_BREAKER_STATE_MAGIC;
}

static void block_breaker_store_state(uint32_t state) {
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_STATE_ADDR = state;
}

static uint32_t block_breaker_load_blocks(void) {
    uint32_t blocks = *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_BLOCKS_ADDR;

    if ((blocks & BLOCK_BREAKER_INITIAL_BLOCKS) == 0u) {
        return 0u;
    }
    return blocks & BLOCK_BREAKER_INITIAL_BLOCKS;
}

static void block_breaker_store_blocks(uint32_t blocks) {
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_BLOCKS_ADDR =
        blocks & BLOCK_BREAKER_INITIAL_BLOCKS;
}

static uint32_t block_breaker_score_for_blocks(uint32_t blocks) {
    uint32_t remaining = blocks & BLOCK_BREAKER_INITIAL_BLOCKS;
    uint32_t count = 0u;

    while (remaining != 0u) {
        count += remaining & 1u;
        remaining >>= 1;
    }
    return 18u - count;
}

static void block_breaker_request_frame_draw(void) {
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_DRAW_PENDING_ADDR = 1u;
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_RENDER_COUNT_ADDR = 0u;
}

static void custom_page_set_visible_count(uint32_t count) {
    uint32_t menu_obj = *(volatile uint32_t *)AIRBREAK_CUSTOM_PAGE_MENU_ADDR;

    if (menu_obj != 0u) {
        *(uint32_t *)(menu_obj + MENU_COUNT_OFF) = count;
    }
}

static uint32_t block_breaker_page_index_for_state(uint32_t state) {
    if ((state & AIRBREAK_BLOCK_BREAKER_ALT_PAGE_BIT) != 0u) {
        return BLOCK_BREAKER_ALT_PAGE_INDEX;
    }
    return BLOCK_BREAKER_PAGE_INDEX;
}

static void block_breaker_copy_text(uint32_t dst_addr, const char *src) {
    volatile char *dst = (volatile char *)dst_addr;

    if (src == 0) {
        src = "";
    }

    while (*src != '\0') {
        *dst = *src;
        dst++;
        src++;
    }
    *dst = '\0';
}

static void copy_custom_about_detail_to_status(void) {
    const char *src = *(const char * const *)ADDR_CUSTOM_ABOUT_DETAIL_PTR_SLOT;

    block_breaker_copy_text(AIRBREAK_BLOCK_BREAKER_STATUS_TEXT_ADDR, src);
}

static void block_breaker_clear_board_row(uint32_t dst_addr) {
    volatile char *dst = (volatile char *)dst_addr;
    uint32_t i;

    dst[0] = '[';
    for (i = 1u; i <= 6u; i++) {
        dst[i] = ' ';
    }
    dst[7] = ']';
    dst[8] = '\0';
}

static void block_breaker_render_strings(uint32_t state) {
    uint32_t blocks = block_breaker_load_blocks();
    uint32_t paddle = state & 0x07u;
    uint32_t ball_x = (state >> 3) & 0x0Fu;
    uint32_t ball_y = (state >> 7) & 0x0Fu;
    uint32_t score = block_breaker_score_for_blocks(blocks);
    uint32_t active = (state >> 12) & 0x01u;
    uint32_t won = (blocks == 0u) ? 1u : 0u;
    uint32_t col;
    volatile char *line;

    block_breaker_clear_board_row(AIRBREAK_BLOCK_BREAKER_STATUS_TEXT_ADDR);
    block_breaker_clear_board_row(AIRBREAK_BLOCK_BREAKER_ROW0_TEXT_ADDR);
    block_breaker_clear_board_row(AIRBREAK_BLOCK_BREAKER_ROW2_TEXT_ADDR);
    line = (volatile char *)AIRBREAK_BLOCK_BREAKER_STATUS_TEXT_ADDR;
    for (col = 0u; col < 6u; col++) {
        if ((blocks & (1u << (12u + col))) != 0u) {
            line[1u + col] = '#';
        }
    }
    line[8] = ' ';
    if (score > 9u) {
        score = 9u;
    }
    line[9] = (char)('0' + score);
    line[10] = '\0';

    ball_x = (ball_x * 6u) >> 4;
    if (ball_x > 5u) {
        ball_x = 5u;
    }
    if (paddle > 4u) {
        paddle = 4u;
    }

    if (ball_y >= 4u && ball_y <= 9u) {
        line = (volatile char *)AIRBREAK_BLOCK_BREAKER_ROW0_TEXT_ADDR;
        line[1u + ball_x] = 'o';
    }

    line = (volatile char *)AIRBREAK_BLOCK_BREAKER_ROW1_TEXT_ADDR;
    block_breaker_clear_board_row(AIRBREAK_BLOCK_BREAKER_ROW1_TEXT_ADDR);
    line[1u + paddle] = '=';
    line[2u + paddle] = '=';
    if (ball_y > 9u) {
        line[1u + ball_x] = 'o';
    }

    if (won != 0u || blocks == 0u) {
        block_breaker_copy_text(AIRBREAK_BLOCK_BREAKER_STATUS_TEXT_ADDR, "Cleared! Fire reset");
        return;
    }

    (void)active;
}

static void lcd_write_cmd(uint32_t cmd) {
    *(volatile uint16_t *)LCD_CMD_ADDR = (uint16_t)cmd;
}

static void lcd_write_data(uint32_t data) {
    *(volatile uint16_t *)LCD_DATA_ADDR = (uint16_t)data;
}

static void lcd_set_window(uint32_t x0, uint32_t y0, uint32_t x1, uint32_t y1) {
    if (x1 >= LCD_WIDTH) {
        x1 = LCD_WIDTH - 1u;
    }
    if (y1 >= LCD_HEIGHT) {
        y1 = LCD_HEIGHT - 1u;
    }

    lcd_write_cmd(0x2Au);
    lcd_write_data(x0 >> 8);
    lcd_write_data(x0 & 0xFFu);
    lcd_write_data(x1 >> 8);
    lcd_write_data(x1 & 0xFFu);

    lcd_write_cmd(0x2Bu);
    lcd_write_data(y0 >> 8);
    lcd_write_data(y0 & 0xFFu);
    lcd_write_data(y1 >> 8);
    lcd_write_data(y1 & 0xFFu);
}

static void lcd_fill_rect(
    uint32_t x,
    uint32_t y,
    uint32_t w,
    uint32_t h,
    uint32_t color
) {
    uint32_t count;

    if (w == 0u || h == 0u || x >= LCD_WIDTH || y >= LCD_HEIGHT) {
        return;
    }
    if ((x + w) > LCD_WIDTH) {
        w = LCD_WIDTH - x;
    }
    if ((y + h) > LCD_HEIGHT) {
        h = LCD_HEIGHT - y;
    }

    lcd_set_window(x, y, x + w - 1u, y + h - 1u);
    lcd_write_cmd(0x2Cu);
    count = w * h;
    while (count != 0u) {
        lcd_write_data(color);
        count--;
    }
}

static uint32_t block_breaker_paddle_x_for_state(uint32_t state) {
    uint32_t paddle = state & 0x07u;
    if (paddle > 4u) {
        paddle = 4u;
    }
    return 20u + (paddle * 38u);
}

static uint32_t block_breaker_ball_x_for_state(uint32_t state) {
    uint32_t active = (state >> 12) & 0x01u;
    uint32_t ball_x = (state >> 3) & 0x0Fu;
    if (ball_x > 15u) {
        ball_x = 15u;
    }
    if (active == 0u) {
        return block_breaker_paddle_x_for_state(state) + 22u;
    }
    return 16u + (ball_x * 13u);
}

static uint32_t block_breaker_ball_y_for_state(uint32_t state) {
    uint32_t active = (state >> 12) & 0x01u;
    uint32_t ball_y = (state >> 7) & 0x0Fu;
    if (active == 0u) {
        return 272u;
    }
    if (ball_y > 15u) {
        ball_y = 15u;
    }
    return 34u + (ball_y * 16u);
}

static void block_breaker_draw_ball_from_state(uint32_t state, uint32_t color) {
    lcd_fill_rect(
        block_breaker_ball_x_for_state(state),
        block_breaker_ball_y_for_state(state),
        8u,
        8u,
        color
    );
}

static void block_breaker_draw_paddle_from_state(uint32_t state, uint32_t color) {
    uint32_t x = block_breaker_paddle_x_for_state(state);
    lcd_fill_rect(x, 282u, 52u, 10u, LCD_COLOR_BLACK);
    if (color != LCD_COLOR_BLACK) {
        lcd_fill_rect(x, 282u, 52u, 8u, color);
        lcd_fill_rect(x, 290u, 52u, 2u, LCD_COLOR_WHITE);
    }
}

static void block_breaker_draw_full_frame(uint32_t state) {
    uint32_t blocks;
    uint32_t score;
    uint32_t won;
    uint32_t col;
    uint32_t row;
    uint32_t color;

    if (!block_breaker_valid_state(state)) {
        return;
    }

    blocks = block_breaker_load_blocks();
    score = block_breaker_score_for_blocks(blocks);
    won = (blocks == 0u) ? 1u : 0u;

    lcd_fill_rect(0u, 0u, LCD_WIDTH, LCD_HEIGHT, LCD_COLOR_BLACK);
    lcd_fill_rect(6u, 18u, 228u, 3u, LCD_COLOR_WHITE);
    lcd_fill_rect(6u, 18u, 3u, 278u, LCD_COLOR_WHITE);
    lcd_fill_rect(231u, 18u, 3u, 278u, LCD_COLOR_WHITE);
    lcd_fill_rect(6u, 293u, 228u, 3u, LCD_COLOR_WHITE);

    for (col = 0u; col < 6u; col++) {
        for (row = 0u; row < 3u; row++) {
            if ((blocks & (1u << ((row * 6u) + col))) != 0u) {
                color = LCD_COLOR_RED;
                if (row == 1u) {
                    color = LCD_COLOR_GREEN;
                } else if (row == 2u) {
                    color = LCD_COLOR_BLUE;
                }
                lcd_fill_rect(16u + (col * 35u), 34u + (row * 18u), 30u, 12u, color);
            }
        }
    }

    block_breaker_draw_paddle_from_state(state, LCD_COLOR_GREEN);
    block_breaker_draw_ball_from_state(state, LCD_COLOR_WHITE);

    for (col = 0u; col < score && col < 9u; col++) {
        lcd_fill_rect(14u + (col * 12u), 7u, 8u, 6u, LCD_COLOR_YELLOW);
    }
    if (won != 0u || blocks == 0u) {
        lcd_fill_rect(60u, 132u, 120u, 10u, LCD_COLOR_GREEN);
        lcd_fill_rect(80u, 150u, 80u, 10u, LCD_COLOR_GREEN);
        lcd_fill_rect(100u, 168u, 40u, 10u, LCD_COLOR_GREEN);
    }

    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_PREV_STATE_ADDR = state;
}

static void block_breaker_draw_incremental_frame(uint32_t state) {
    uint32_t old_state = *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_PREV_STATE_ADDR;

    if (!block_breaker_valid_state(old_state) ||
        ((old_state ^ state) & 0x00001007u) != 0u) {
        block_breaker_draw_full_frame(state);
        return;
    }

    block_breaker_draw_ball_from_state(old_state, LCD_COLOR_BLACK);
    if ((old_state & 0x07u) != (state & 0x07u)) {
        block_breaker_draw_paddle_from_state(old_state, LCD_COLOR_BLACK);
        block_breaker_draw_paddle_from_state(state, LCD_COLOR_GREEN);
    }
    block_breaker_draw_ball_from_state(state, LCD_COLOR_WHITE);
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_PREV_STATE_ADDR = state;
}

__attribute__((used, noinline))
static uint32_t block_breaker_try_skip_menu_entry_render(void) {
    volatile uint32_t *pending = (volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_DRAW_PENDING_ADDR;
    volatile uint32_t *render_count = (volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_RENDER_COUNT_ADDR;
    uint32_t state;
    uint32_t count;

    state = *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_STATE_ADDR;
    if (!block_breaker_valid_state(state)) {
        *pending = 0u;
        *render_count = 0u;
        return 0u;
    }

    count = *render_count + 1u;
    *render_count = count;
    return 1u;
}

__attribute__((used, naked))
void patch_menu_render_entry_hook(void) {
    __asm volatile(
        "push {r0, r1, r2, r3, r4, lr}\n"
        "bl block_breaker_try_skip_menu_entry_render\n"
        "cmp r0, #0\n"
        "beq 1f\n"
        "pop {r0, r1, r2, r3, r4, pc}\n"
        "1:\n"
        "pop {r0, r1, r2, r3, r4, lr}\n"
        "push {lr}\n"
        "ldr lr, =patch_menu_render_entry_tail\n"
        "orr lr, lr, #1\n"
        "push {r3, r4, r5, r6, r7, lr}\n"
        "mov r4, r0\n"
        "ldr pc, =0x08064FC3\n"
    );
}

__attribute__((used, naked))
void patch_menu_render_entry_tail(void) {
    __asm volatile(
        "pop {pc}\n"
    );
}

__attribute__((used, naked))
uint32_t patch_event_set_hook(uint32_t event_id, uint32_t value) {
    (void)event_id;
    (void)value;
    __asm volatile(
        "ldr r2, =%c[state_addr]\n"
        "ldr r2, [r2]\n"
        "lsrs r3, r2, #24\n"
        "cmp r3, #0xB4\n"
        "bne 3f\n"
        "ldr r3, =%c[event_page_row]\n"
        "cmp r0, r3\n"
        "beq 2f\n"
        "ldr r3, =%c[event_selected_row]\n"
        "cmp r0, r3\n"
        "beq 2f\n"
        "ldr r3, =%c[event_page_index]\n"
        "cmp r0, r3\n"
        "bne 3f\n"
        "ldr r3, =%c[alt_page_bit]\n"
        "tst r2, r3\n"
        "movs r3, #%c[block_page]\n"
        "beq 1f\n"
        "movs r3, #%c[block_alt_page]\n"
        "1:\n"
        "cmp r1, r3\n"
        "beq 3f\n"
        "2:\n"
        "movs r0, #0\n"
        "bx lr\n"
        "3:\n"
        "push {r4, lr}\n"
        "sub sp, sp, #64\n"
        "ldr pc, =0x08066E83\n"
        :
        : [state_addr] "i" (AIRBREAK_BLOCK_BREAKER_STATE_ADDR),
          [event_page_index] "i" (EVENT_PAGE_INDEX),
          [event_page_row] "i" (EVENT_PAGE_ROW),
          [event_selected_row] "i" (EVENT_SELECTED_ROW),
          [alt_page_bit] "i" (AIRBREAK_BLOCK_BREAKER_ALT_PAGE_BIT),
          [block_page] "i" (BLOCK_BREAKER_PAGE_INDEX),
          [block_alt_page] "i" (BLOCK_BREAKER_ALT_PAGE_INDEX)
        : "r2", "r3"
    );
}

static uint32_t block_breaker_encoder_phase(void) {
    uint32_t idr = *(volatile uint32_t *)GPIOF_IDR_ADDR;
    uint32_t phase = 0u;

    if ((idr & GPIOF_ENCODER_A_MASK) != 0u) {
        phase |= 2u;
    }
    if ((idr & GPIOF_ENCODER_B_MASK) != 0u) {
        phase |= 1u;
    }
    return phase;
}

static uint32_t block_breaker_encoder_packed_state(uint32_t phase, uint32_t acc) {
    return (phase & 0x03u) | ((acc & 0x0Fu) << 8);
}

static void block_breaker_clear_rotary_provider(void) {
    uint32_t obj = AIRBREAK_ROTARY_PROVIDER_ADDR;

    *(volatile int16_t *)(obj + ROTARY_COUNT_OFF) = 0;
    *(volatile uint8_t *)(obj + ROTARY_DIRECTION_OFF) = 0u;
    *(volatile uint8_t *)(obj + ROTARY_CHANGED_OFF) = 0u;
    *(volatile uint8_t *)(obj + ROTARY_PENDING_OFF) = 0u;
}

static uint32_t block_breaker_rotary_provider_command(void) {
    uint32_t obj = AIRBREAK_ROTARY_PROVIDER_ADDR;
    int16_t count = *(volatile int16_t *)(obj + ROTARY_COUNT_OFF);
    uint32_t direction = *(volatile uint8_t *)(obj + ROTARY_DIRECTION_OFF);
    uint32_t changed = *(volatile uint8_t *)(obj + ROTARY_CHANGED_OFF);
    uint32_t command = 0u;

    if (count > 0) {
        command = BLOCK_BREAKER_CMD_RIGHT;
    } else if (count < 0) {
        command = BLOCK_BREAKER_CMD_LEFT;
    } else if (changed != 0u && direction == 1u) {
        command = BLOCK_BREAKER_CMD_RIGHT;
    } else if (changed != 0u && direction == 2u) {
        command = BLOCK_BREAKER_CMD_LEFT;
    }

    if (command != 0u) {
        block_breaker_clear_rotary_provider();
    }
    return command;
}

static uint32_t block_breaker_raw_encoder_command(volatile uint32_t *input_state) {
    uint32_t packed = *input_state;
    uint32_t previous = packed & 0x03u;
    uint32_t acc = (packed >> 8) & 0x0Fu;
    uint32_t current = block_breaker_encoder_phase();
    uint32_t cw = 0u;
    uint32_t ccw = 0u;

    if (acc > BLOCK_BREAKER_ENCODER_ACC_MAX) {
        acc = BLOCK_BREAKER_ENCODER_ACC_NEUTRAL;
    }
    if (previous == current) {
        return 0u;
    }

    if ((previous == 3u && current == 1u) ||
        (previous == 1u && current == 0u) ||
        (previous == 0u && current == 2u) ||
        (previous == 2u && current == 3u)) {
        cw = 1u;
    } else if ((previous == 3u && current == 2u) ||
        (previous == 2u && current == 0u) ||
        (previous == 0u && current == 1u) ||
        (previous == 1u && current == 3u)) {
        ccw = 1u;
    } else {
        acc = BLOCK_BREAKER_ENCODER_ACC_NEUTRAL;
    }

    if (cw != 0u && acc < BLOCK_BREAKER_ENCODER_ACC_MAX) {
        acc++;
    } else if (ccw != 0u && acc > 0u) {
        acc--;
    }

    if (current == 3u && acc >= BLOCK_BREAKER_ENCODER_ACC_MAX) {
        *input_state = block_breaker_encoder_packed_state(current, BLOCK_BREAKER_ENCODER_ACC_NEUTRAL);
        return BLOCK_BREAKER_CMD_RIGHT;
    }
    if (current == 3u && acc == 0u) {
        *input_state = block_breaker_encoder_packed_state(current, BLOCK_BREAKER_ENCODER_ACC_NEUTRAL);
        return BLOCK_BREAKER_CMD_LEFT;
    }

    *input_state = block_breaker_encoder_packed_state(current, acc);
    return 0u;
}

__attribute__((used, noinline))
static uint32_t block_breaker_post_render_wait_active(uint32_t wait_base, uint32_t wait_ticks) {
    fn_wait_predicate_t wait_fn = (fn_wait_predicate_t)(ADDR_POST_RENDER_WAIT | 1u);
    fn_event_set_t event_set_fn = (fn_event_set_t)(ADDR_EVENT_SET | 1u);
    volatile uint32_t *last_button = (volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_BUTTON_STATE_ADDR;
    volatile uint32_t *pending = (volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_DRAW_PENDING_ADDR;
    volatile uint32_t *render_count = (volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_RENDER_COUNT_ADDR;
    volatile uint32_t *input_state = (volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_LAST_ROW_ADDR;
    volatile uint32_t *tick_counter = (volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_TICK_COUNTER_ADDR;
    uint32_t state = *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_STATE_ADDR;
    uint32_t done = 0u;
    uint32_t pressed;
    uint32_t home_pressed;
    uint32_t previous_buttons;
    uint32_t command = 0u;
    uint32_t ticked = 0u;
    uint32_t stock_rendered;
    uint32_t tick;

    if (!block_breaker_valid_state(state)) {
        *last_button = 0u;
        *input_state = 0u;
        *tick_counter = 0u;
        return wait_fn(wait_base, wait_ticks);
    }

    previous_buttons = *last_button;
    custom_page_set_visible_count(CUSTOM_PAGE_VISIBLE_BLOCK_BREAKER_COUNT);
    home_pressed = ((*(volatile uint32_t *)GPIOG_IDR_ADDR & GPIOG_HOME_BUTTON_MASK) == 0u) ? 1u : 0u;
    if (home_pressed != 0u && (previous_buttons & 0x02u) == 0u) {
        patch_custom_about_back_exec(0u);
        return wait_fn(wait_base, wait_ticks);
    }

    done = wait_fn(wait_base, wait_ticks);

    command = block_breaker_rotary_provider_command();
    if (command != 0u) {
        *input_state = block_breaker_encoder_packed_state(
            block_breaker_encoder_phase(),
            BLOCK_BREAKER_ENCODER_ACC_NEUTRAL
        );
    } else {
        command = block_breaker_raw_encoder_command(input_state);
    }
    if (command != 0u) {
        state = block_breaker_apply_command(state, command);
        block_breaker_store_state(state);
    }

    pressed = ((*(volatile uint32_t *)GPIOG_IDR_ADDR & GPIOG_ENCODER_BUTTON_MASK) == 0u) ? 1u : 0u;
    if (pressed != 0u && (previous_buttons & 0x01u) == 0u) {
        state = block_breaker_apply_command(state, BLOCK_BREAKER_CMD_FIRE);
        block_breaker_store_state(state);
        command = BLOCK_BREAKER_CMD_FIRE;
    }

    if (((state >> 12) & 0x01u) != 0u) {
        tick = *tick_counter + 1u;
        if (tick >= BLOCK_BREAKER_TICK_DIVISOR) {
            tick = 0u;
            state = block_breaker_apply_command(state, BLOCK_BREAKER_CMD_TICK);
            block_breaker_store_state(state);
            ticked = 1u;
        }
        *tick_counter = tick;
    } else {
        *tick_counter = 0u;
    }

    stock_rendered = *render_count;
    if (command != 0u || ticked != 0u || *pending != 0u || stock_rendered != 0u) {
        if (ticked != 0u && command == 0u && *pending == 0u && stock_rendered == 0u) {
            block_breaker_draw_incremental_frame(state);
        } else {
            if (command != 0u || ticked != 0u || *pending != 0u) {
                render_airbreak_page_text_for_state(state);
                event_set_fn(EVENT_PAGE_INDEX, block_breaker_page_index_for_state(state));
            }
            block_breaker_draw_full_frame(state);
        }
        *pending = 0u;
        *render_count = 0u;
    }
    *last_button = (pressed != 0u ? 0x01u : 0u) |
        (home_pressed != 0u ? 0x02u : 0u);
    return done;
}

__attribute__((used, naked))
uint32_t patch_block_breaker_post_render_wait_hook(uint32_t wait_base, uint32_t wait_ticks) {
    (void)wait_base;
    (void)wait_ticks;
    __asm volatile(
        "ldr r2, =%c[state_addr]\n"
        "ldr r2, [r2]\n"
        "lsrs r2, r2, #24\n"
        "cmp r2, #0xB4\n"
        "beq 1f\n"
        "ldr r2, =%c[wait_fn]\n"
        "bx r2\n"
        "1:\n"
        "b block_breaker_post_render_wait_active\n"
        :
        : [state_addr] "i" (AIRBREAK_BLOCK_BREAKER_STATE_ADDR),
          [wait_fn] "i" (ADDR_POST_RENDER_WAIT | 1u)
        : "r2"
    );
}

static uint32_t block_breaker_apply_command(uint32_t state, uint32_t command) {
    uint32_t blocks;
    uint32_t paddle;
    uint32_t ball_x;
    uint32_t ball_y;
    uint32_t dx;
    uint32_t dy;
    uint32_t active;
    uint32_t page_flip;
    uint32_t nx;
    uint32_t ny;
    uint32_t block_row;
    uint32_t block_col;
    uint32_t block_idx;
    uint32_t paddle_left;
    uint32_t paddle_right;

    if (!block_breaker_valid_state(state)) {
        block_breaker_store_blocks(BLOCK_BREAKER_INITIAL_BLOCKS);
        state = block_breaker_initial_state();
    }

    blocks = block_breaker_load_blocks();
    paddle = state & 0x07u;
    ball_x = (state >> 3) & 0x0Fu;
    ball_y = (state >> 7) & 0x0Fu;
    dx = (state >> 11) & 0x01u;
    dy = (state & AIRBREAK_BLOCK_BREAKER_DY_BIT) != 0u;
    active = (state >> 12) & 0x01u;
    page_flip = (state & AIRBREAK_BLOCK_BREAKER_ALT_PAGE_BIT) ^
        AIRBREAK_BLOCK_BREAKER_ALT_PAGE_BIT;

    if (command == BLOCK_BREAKER_CMD_LEFT) {
        if (paddle > 0u) {
            paddle--;
        }
        if (active == 0u) {
            ball_x = block_breaker_paddle_ball_x(paddle);
            ball_y = 15u;
        }
    } else if (command == BLOCK_BREAKER_CMD_RIGHT) {
        if (paddle < 4u) {
            paddle++;
        }
        if (active == 0u) {
            ball_x = block_breaker_paddle_ball_x(paddle);
            ball_y = 15u;
        }
    } else if (command == BLOCK_BREAKER_CMD_FIRE) {
        if (blocks == 0u) {
            block_breaker_store_blocks(BLOCK_BREAKER_INITIAL_BLOCKS);
            return block_breaker_initial_state() | page_flip;
        }

        if (active == 0u) {
            active = 1u;
            dx = 1u;
            dy = 0u;
            ball_x = block_breaker_paddle_ball_x(paddle);
            ball_y = 15u;
        }
    }

    if (active != 0u && command == BLOCK_BREAKER_CMD_TICK) {
        nx = ball_x;
        ny = ball_y;

        if (dx == 0u) {
            if (nx == 0u) {
                dx = 1u;
                nx = 1u;
            } else {
                nx--;
            }
        } else {
            if (nx >= 15u) {
                dx = 0u;
                nx = 14u;
            } else {
                nx++;
            }
        }

        if (dy == 0u) {
            if (ny == 0u) {
                dy = 1u;
                ny = 1u;
            } else {
                ny--;
            }
        } else {
            ny++;
        }

        if (ny <= 2u) {
            block_row = ny;
            block_col = (nx * 6u) >> 4;
            if (block_col > 5u) {
                block_col = 5u;
            }
            block_idx = (block_row * 6u) + block_col;
            if ((blocks & (1u << block_idx)) != 0u) {
                blocks &= ~(1u << block_idx);
                block_breaker_store_blocks(blocks);
                block_breaker_request_frame_draw();
                dy = 1u;
                ny = block_row + 1u;
                if (blocks == 0u) {
                    active = 0u;
                }
            } else if (ny == 0u) {
                dy = 1u;
                ny = 1u;
            }
        }

        if (ny >= 15u) {
            paddle_left = paddle * 3u;
            paddle_right = paddle_left + 4u;
            if (nx >= paddle_left && nx <= paddle_right) {
                dy = 0u;
                ny = 14u;
                if (nx <= (paddle_left + 1u)) {
                    dx = 0u;
                } else if (nx >= (paddle_right - 1u)) {
                    dx = 1u;
                }
            } else {
                active = 0u;
                dx = 1u;
                dy = 0u;
                nx = block_breaker_paddle_ball_x(paddle);
                ny = 15u;
            }
        }

        ball_x = nx;
        ball_y = ny;
    }

    return block_breaker_pack_state(
        paddle,
        ball_x,
        ball_y,
        dx,
        dy,
        active
    ) | page_flip;
}

static void render_airbreak_page_text_for_state(uint32_t state) {
    if (block_breaker_valid_state(state)) {
        block_breaker_copy_text(AIRBREAK_CUSTOM_PAGE_TITLE_TEXT_ADDR, "Block Breaker");
        block_breaker_render_strings(state);
        return;
    }

    block_breaker_copy_text(AIRBREAK_CUSTOM_PAGE_TITLE_TEXT_ADDR, "Custom About");
    copy_custom_about_detail_to_status();
}

static void store_block_breaker_return_state(uint32_t return_page, uint32_t return_row) {
    volatile uint32_t *state = (volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_RETURN_STATE_ADDR;

    *state = AIRBREAK_RETURN_STATE_MAGIC |
        ((return_page & 0xFFu) << 16) |
        (return_row & 0xFFFFu);
}

__attribute__((used, noinline))
static uint32_t patch_block_breaker_entry_exec(uint32_t action_obj) {
    fn_event_get_t event_get_fn = (fn_event_get_t)(ADDR_EVENT_GET | 1u);
    fn_event_set_t event_set_fn = (fn_event_set_t)(ADDR_EVENT_SET | 1u);
    uint32_t return_page = 0u;
    uint32_t return_row = event_get_fn(EVENT_SELECTED_ROW);
    uint32_t state = block_breaker_initial_state();

    if (action_obj != 0u) {
        return_page = normalize_my_options_return_page(*(uint8_t *)(action_obj + 4u));
    }
    if (return_page == 0u) {
        return_page = normalize_my_options_return_page(event_get_fn(EVENT_PAGE_INDEX));
    }
    if (return_page == 0u) {
        return_page = RETURN_PAGE_COMPACT_MY_OPTIONS;
    }

    store_block_breaker_return_state(return_page, return_row);
    block_breaker_copy_text(AIRBREAK_CUSTOM_PAGE_TITLE_TEXT_ADDR, "Block Breaker");
    custom_page_set_visible_count(CUSTOM_PAGE_VISIBLE_BLOCK_BREAKER_COUNT);
    event_set_fn(EVENT_PAGE_INDEX, block_breaker_page_index_for_state(state));
    event_set_fn(EVENT_PAGE_ROW, 0u);
    block_breaker_store_blocks(BLOCK_BREAKER_INITIAL_BLOCKS);
    block_breaker_store_state(state);
    block_breaker_render_strings(state);
    block_breaker_clear_rotary_provider();
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_LAST_ROW_ADDR =
        block_breaker_encoder_packed_state(
            block_breaker_encoder_phase(),
            BLOCK_BREAKER_ENCODER_ACC_NEUTRAL
        );
    *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_TICK_COUNTER_ADDR = 0u;
    block_breaker_request_frame_draw();
    return 0u;
}

__attribute__((used, aligned(4)))
static const fn_action_vfunc_t CUSTOM_ABOUT_ENTRY_ACTION_VTABLE[4] = {
    (fn_action_vfunc_t)ADDR_NAV_ACTION_VFUNC_0,
    (fn_action_vfunc_t)0u,
    (fn_action_vfunc_t)ADDR_NAV_ACTION_VFUNC_8,
    patch_custom_about_entry_exec,
};

__attribute__((used, aligned(4)))
static const fn_action_vfunc_t CUSTOM_ABOUT_BACK_ACTION_VTABLE[4] = {
    (fn_action_vfunc_t)ADDR_NAV_ACTION_VFUNC_0,
    (fn_action_vfunc_t)0u,
    (fn_action_vfunc_t)ADDR_NAV_ACTION_VFUNC_8,
    patch_custom_about_back_exec,
};

__attribute__((used, aligned(4)))
static const fn_action_vfunc_t BLOCK_BREAKER_ENTRY_ACTION_VTABLE[4] = {
    (fn_action_vfunc_t)ADDR_NAV_ACTION_VFUNC_0,
    (fn_action_vfunc_t)0u,
    (fn_action_vfunc_t)ADDR_NAV_ACTION_VFUNC_8,
    patch_block_breaker_entry_exec,
};

static uint32_t menu_has_room(uint32_t menu_obj) {
    uint32_t count = *(uint32_t *)(menu_obj + MENU_COUNT_OFF);
    uint32_t cap = *(uint32_t *)(menu_obj + MENU_CAPACITY_OFF);
    return count < cap;
}

static uint32_t normalize_my_options_return_page(uint32_t return_page) {
    return_page &= 0xFFu;

    if (return_page == RETURN_PAGE_PLUS_MY_OPTIONS ||
        return_page == RETURN_PAGE_COMPACT_MY_OPTIONS) {
        return return_page;
    }
    return 0u;
}

static uint32_t custom_about_return_page_for_hook(uint32_t hook_return_addr) {
    hook_return_addr &= ~1u;

    if (hook_return_addr == HOOK_RETURN_PLUS_MY_OPTIONS) {
        return RETURN_PAGE_PLUS_MY_OPTIONS;
    }
    if (hook_return_addr == HOOK_RETURN_COMPACT_MY_OPTIONS) {
        return RETURN_PAGE_COMPACT_MY_OPTIONS;
    }
    return 0u;
}

static uint32_t custom_about_return_page_for_current_context(uint32_t hook_return_addr) {
    fn_event_get_t event_get_fn = (fn_event_get_t)(ADDR_EVENT_GET | 1u);
    uint32_t return_page = custom_about_return_page_for_hook(hook_return_addr);

    if (return_page != 0u) {
        return return_page;
    }

    return_page = normalize_my_options_return_page(event_get_fn(EVENT_PAGE_INDEX));
    if (return_page != 0u) {
        return return_page;
    }

    return RETURN_PAGE_COMPACT_MY_OPTIONS;
}

__attribute__((used, noinline))
static uint32_t append_nav_row_if_room(
    fn_alloc_t alloc_fn,
    fn_menu_append_t menu_append_fn,
    fn_nav_action_ctor_t nav_action_ctor_fn,
    fn_nav_row_ctor_t nav_row_ctor_fn,
    uint32_t menu_obj,
    uint32_t p3,
    uint32_t p4,
    uint32_t label_id,
    uint32_t page_index,
    uint32_t page_row
) {
    uint32_t nav_action;
    uint32_t item_obj;

    if (!menu_has_room(menu_obj)) {
        return 0u;
    }

    nav_action = alloc_fn(NAV_ACTION_ALLOC_SIZE);
    if (nav_action == 0u) {
        return 0u;
    }
    nav_action = nav_action_ctor_fn(nav_action, page_index, page_row);

    item_obj = alloc_fn(NAV_ROW_ALLOC_SIZE);
    if (item_obj == 0u) {
        return 0u;
    }

    item_obj = nav_row_ctor_fn(
        item_obj,
        label_id,
        0u,
        nav_action,
        NAV_ROW_STYLE
    );
    if (item_obj == 0u) {
        return 0u;
    }

    menu_append_fn(menu_obj, item_obj, p3, p4);
    return 1u;
}

__attribute__((used, noinline))
static uint32_t append_custom_about_nav_row_if_room(
    fn_alloc_t alloc_fn,
    fn_menu_append_t menu_append_fn,
    fn_nav_row_ctor_t nav_row_ctor_fn,
    uint32_t menu_obj,
    uint32_t p3,
    uint32_t p4,
    uint32_t return_page
) {
    uint32_t nav_action;
    uint32_t item_obj;

    if (!menu_has_room(menu_obj)) {
        return 0u;
    }

    nav_action = alloc_fn(NAV_ACTION_ALLOC_SIZE);
    if (nav_action == 0u) {
        return 0u;
    }
    *(uint32_t *)nav_action = (uint32_t)(uintptr_t)CUSTOM_ABOUT_ENTRY_ACTION_VTABLE;
    *(uint8_t *)(nav_action + 4u) = (uint8_t)return_page;
    *(uint16_t *)(nav_action + 6u) = 0u;

    item_obj = alloc_fn(NAV_ROW_ALLOC_SIZE);
    if (item_obj == 0u) {
        return 0u;
    }

    item_obj = nav_row_ctor_fn(
        item_obj,
        CUSTOM_ABOUT_LABEL_ID,
        0u,
        nav_action,
        NAV_ROW_STYLE
    );
    if (item_obj == 0u) {
        return 0u;
    }

    menu_append_fn(menu_obj, item_obj, p3, p4);
    return 1u;
}

__attribute__((used, noinline))
static uint32_t append_block_breaker_nav_row_if_room(
    fn_alloc_t alloc_fn,
    fn_menu_append_t menu_append_fn,
    fn_nav_row_ctor_t nav_row_ctor_fn,
    uint32_t menu_obj,
    uint32_t p3,
    uint32_t p4,
    uint32_t return_page
) {
    uint32_t nav_action;
    uint32_t item_obj;

    if (!menu_has_room(menu_obj)) {
        return 0u;
    }

    nav_action = alloc_fn(NAV_ACTION_ALLOC_SIZE);
    if (nav_action == 0u) {
        return 0u;
    }
    *(uint32_t *)nav_action = (uint32_t)(uintptr_t)BLOCK_BREAKER_ENTRY_ACTION_VTABLE;
    *(uint8_t *)(nav_action + 4u) = (uint8_t)return_page;
    *(uint16_t *)(nav_action + 6u) = 0u;

    item_obj = alloc_fn(NAV_ROW_ALLOC_SIZE);
    if (item_obj == 0u) {
        return 0u;
    }

    item_obj = nav_row_ctor_fn(
        item_obj,
        BLOCK_BREAKER_LABEL_ID,
        0u,
        nav_action,
        NAV_ROW_STYLE
    );
    if (item_obj == 0u) {
        return 0u;
    }

    menu_append_fn(menu_obj, item_obj, p3, p4);
    return 1u;
}

__attribute__((used, noinline))
static uint32_t append_simple_text_row_if_room(
    fn_alloc_t alloc_fn,
    fn_menu_append_t menu_append_fn,
    fn_simple_text_row_ctor_t simple_text_row_ctor_fn,
    uint32_t menu_obj,
    uint32_t p3,
    uint32_t p4,
    uint32_t label_id
) {
    uint32_t item_obj;

    if (!menu_has_room(menu_obj)) {
        return 0u;
    }

    item_obj = alloc_fn(SIMPLE_TEXT_ROW_ALLOC_SIZE);
    if (item_obj == 0u) {
        return 0u;
    }

    item_obj = simple_text_row_ctor_fn(item_obj, label_id);
    if (item_obj == 0u) {
        return 0u;
    }

    menu_append_fn(menu_obj, item_obj, p3, p4);
    return 1u;
}

static void install_first_row_back_action(uint32_t menu_obj, const fn_action_vfunc_t *vtable) {
    uint32_t items_obj;
    uint32_t back_row;
    uint32_t back_action;

    if (*(uint32_t *)(menu_obj + MENU_COUNT_OFF) == 0u) {
        return;
    }

    items_obj = *(uint32_t *)(menu_obj + MENU_ITEMS_OFF);
    if (items_obj == 0u) {
        return;
    }

    back_row = *(uint32_t *)items_obj;
    if (back_row == 0u) {
        return;
    }

    back_action = *(uint32_t *)(back_row + NAV_ROW_ACTION_OFF);
    if (back_action == 0u) {
        return;
    }

    *(uint32_t *)back_action = (uint32_t)(uintptr_t)vtable;
}

static void install_custom_about_back_action(uint32_t menu_obj) {
    install_first_row_back_action(menu_obj, CUSTOM_ABOUT_BACK_ACTION_VTABLE);
}

__attribute__((used, section(".patch_target_fn"), noinline))
uint32_t patch_my_options_essentials_append_hook(
    uint32_t menu_obj,
    uint32_t original_item,
    uint32_t p3,
    uint32_t p4
) {
    uint32_t hook_return_addr = (uint32_t)(uintptr_t)__builtin_return_address(0);
    fn_alloc_t alloc_fn = (fn_alloc_t)(ADDR_ALLOC | 1u);
    fn_menu_append_t menu_append_fn = (fn_menu_append_t)(ADDR_MENU_APPEND | 1u);
    fn_nav_action_ctor_t nav_action_ctor_fn = (fn_nav_action_ctor_t)(ADDR_NAV_ACTION_CTOR | 1u);
    fn_nav_row_ctor_t nav_row_ctor_fn = (fn_nav_row_ctor_t)(ADDR_NAV_ROW_CTOR | 1u);

    uint32_t custom_about_return_page =
        custom_about_return_page_for_current_context(hook_return_addr);
    uint32_t append_ret;
    volatile uint32_t append_status;

    append_ret = menu_append_fn(menu_obj, original_item, p3, p4);

    if (menu_obj == 0u) {
        return append_ret;
    }

    append_status = append_block_breaker_nav_row_if_room(
        alloc_fn,
        menu_append_fn,
        nav_row_ctor_fn,
        menu_obj,
        p3,
        p4,
        custom_about_return_page
    );

    append_status = append_custom_about_nav_row_if_room(
        alloc_fn,
        menu_append_fn,
        nav_row_ctor_fn,
        menu_obj,
        p3,
        p4,
        custom_about_return_page
    );

    append_status = append_nav_row_if_room(
        alloc_fn,
        menu_append_fn,
        nav_action_ctor_fn,
        nav_row_ctor_fn,
        menu_obj,
        p3,
        p4,
        CLINICAL_MODE_LABEL_ID,
        CLINICAL_MODE_PAGE_INDEX,
        CLINICAL_MODE_PAGE_ROW
    );
    (void)append_status;
    return append_ret;
}

__attribute__((used, section(".patch_custom_page_seed_fn"), noinline))
uint32_t patch_custom_about_page_seed_hook(
    uint32_t menu_obj,
    uint32_t original_item,
    uint32_t p3,
    uint32_t p4
) {
    fn_alloc_t alloc_fn = (fn_alloc_t)(ADDR_ALLOC | 1u);
    fn_menu_append_t menu_append_fn = (fn_menu_append_t)(ADDR_MENU_APPEND | 1u);
    fn_nav_row_ctor_t nav_row_ctor_fn = (fn_nav_row_ctor_t)(ADDR_NAV_ROW_CTOR | 1u);
    fn_simple_text_row_ctor_t simple_text_row_ctor_fn =
        (fn_simple_text_row_ctor_t)(ADDR_SIMPLE_TEXT_ROW_CTOR | 1u);
    uint32_t state = *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_STATE_ADDR;
    uint32_t block_active = block_breaker_valid_state(state);
    uint32_t append_ret;
    volatile uint32_t append_status;

    append_ret = menu_append_fn(menu_obj, original_item, p3, p4);

    if (menu_obj == 0u) {
        return append_ret;
    }

    install_custom_about_back_action(menu_obj);
    *(volatile uint32_t *)AIRBREAK_CUSTOM_PAGE_MENU_ADDR = menu_obj;

    if (block_active) {
        *(uint32_t *)(menu_obj + MENU_COUNT_OFF) = 0u;
    } else if (*(uint32_t *)(menu_obj + MENU_COUNT_OFF) > 1u) {
        *(uint32_t *)(menu_obj + MENU_COUNT_OFF) = 1u;
    }

    render_airbreak_page_text_for_state(state);

    if (!block_active) {
        append_status = append_simple_text_row_if_room(
            alloc_fn,
            menu_append_fn,
            simple_text_row_ctor_fn,
            menu_obj,
            p3,
            p4,
            BLOCK_BREAKER_STATUS_ID
        );
    }

    (void)append_status;
    return append_ret;
}

__attribute__((used, section(".patch_custom_page_tail_fn"), noinline))
uint32_t patch_custom_about_page_tail_hook(
    uint32_t menu_obj,
    uint32_t original_item,
    uint32_t p3,
    uint32_t p4
) {
    (void)original_item;
    (void)p3;
    uint32_t state = *(volatile uint32_t *)AIRBREAK_BLOCK_BREAKER_STATE_ADDR;
    uint32_t block_active = block_breaker_valid_state(state);

    if (menu_obj == 0u) {
        return p4;
    }

    *(volatile uint32_t *)AIRBREAK_CUSTOM_PAGE_MENU_ADDR = menu_obj;
    install_custom_about_back_action(menu_obj);
    render_airbreak_page_text_for_state(state);
    custom_page_set_visible_count(
        block_active ?
            CUSTOM_PAGE_VISIBLE_BLOCK_BREAKER_COUNT :
            CUSTOM_PAGE_VISIBLE_CUSTOM_COUNT
    );
    if (block_active) {
        block_breaker_request_frame_draw();
    }
    return p4;
}
