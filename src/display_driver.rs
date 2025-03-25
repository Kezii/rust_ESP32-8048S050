use std::sync::Arc;

use esp_idf_hal::{
    gpio::AnyOutputPin,
    ledc::{config, LedcDriver, LedcTimerDriver, LEDC},
    sys::{
        esp_lcd_panel_disp_on_off, esp_lcd_panel_draw_bitmap, esp_lcd_panel_handle_t,
        esp_lcd_panel_t, esp_lcd_rgb_panel_config_t, esp_lcd_rgb_panel_config_t__bindgen_ty_1,
        esp_lcd_rgb_timing_t, esp_lcd_rgb_timing_t__bindgen_ty_1,
        soc_periph_lcd_clk_src_t_LCD_CLK_SRC_PLL160M,
    },
};

use esp_idf_hal::prelude::*;

use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct EspParallelLCD {
    pub panel: esp_lcd_panel_handle_t,
    backlight: Option<LedcDriver<'static>>,
}

impl EspParallelLCD {
    pub fn new() -> Self {
        let panel_config = Arc::new(esp_lcd_rgb_panel_config_t {
            clk_src: soc_periph_lcd_clk_src_t_LCD_CLK_SRC_PLL160M,
            timings: esp_lcd_rgb_timing_t {
                pclk_hz: (16 * 1000 * 1000),
                h_res: SCREEN_WIDTH as u32,
                v_res: SCREEN_HEIGHT as u32,
                hsync_pulse_width: 4,
                hsync_back_porch: 8,
                hsync_front_porch: 8,
                vsync_pulse_width: 4,
                vsync_back_porch: 8,
                vsync_front_porch: 8,
                flags: {
                    let mut timings_flags = esp_lcd_rgb_timing_t__bindgen_ty_1::default();

                    timings_flags.set_de_idle_high(0);
                    timings_flags.set_pclk_active_neg(1);
                    timings_flags.set_pclk_idle_high(0);
                    timings_flags
                },
            },
            data_width: 16,
            bits_per_pixel: 0, //xxx
            num_fbs: 0,        //xxx
            bounce_buffer_size_px: 0,
            sram_trans_align: 8,
            psram_trans_align: 64,
            hsync_gpio_num: 39,
            vsync_gpio_num: 41,
            de_gpio_num: 40,
            pclk_gpio_num: 42,
            disp_gpio_num: -1,
            data_gpio_nums: [
                8, 3, 46, 9, 1, 5, 6, 7, 15, 16, 4, 45, 48, 47, 21,
                14,
                //        15, 16, 4, 45, 48, 47, 21, 14, 8, 3, 46, 9, 1, 5, 6, 7
            ],
            flags: {
                let mut panel_flags = esp_lcd_rgb_panel_config_t__bindgen_ty_1::default();

                panel_flags.set_disp_active_low(0);
                panel_flags.set_fb_in_psram(1);
                //panel_flags.set_refresh_on_demand(1);

                panel_flags
            },
        });

        let panel = prepare_lcd_panel(&panel_config);
        Self {
            panel,
            backlight: None,
        }
    }

    pub fn prepare_backlight(&mut self, ledc: LEDC, gpio: AnyOutputPin) {
        let config = config::TimerConfig::new().frequency(25.kHz().into());
        let timer = LedcTimerDriver::new(ledc.timer0, &config).unwrap();
        let mut backlight_pwm = LedcDriver::new(ledc.channel0, timer, gpio).unwrap();

        backlight_pwm
            .set_duty(backlight_pwm.get_max_duty())
            .unwrap();

        self.backlight = Some(backlight_pwm);
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn draw_bitmap(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        data: *mut std::ffi::c_void,
    ) {
        //unsafe { esp_idf_svc::sys::esp_lcd_panel_draw_bitmap(self.panel, x, y, width, height, data) };
        unsafe {
            esp_lcd_panel_draw_bitmap(self.panel, x, y, width, height, data);
        }
    }

    pub fn restart(&self) {
        unsafe { esp_idf_svc::sys::esp_lcd_rgb_panel_restart(self.panel) };
    }
}

fn prepare_lcd_panel(panel_config: &esp_lcd_rgb_panel_config_t) -> esp_lcd_panel_handle_t {
    let mut esp_lcd_panel = esp_lcd_panel_t::default();
    let mut ret_panel = &mut esp_lcd_panel as *mut esp_idf_svc::sys::esp_lcd_panel_t;

    unsafe { esp_idf_svc::sys::esp_lcd_new_rgb_panel(panel_config, &mut ret_panel) };

    unsafe { esp_idf_svc::sys::esp_lcd_panel_reset(ret_panel) };

    unsafe { esp_idf_svc::sys::esp_lcd_panel_init(ret_panel) };

    unsafe { esp_lcd_panel_disp_on_off(ret_panel, true) };

    ret_panel
}

#[derive(Debug)]
pub enum FbWriteError {
    Error,
}
pub trait FramebufferTarget {
    fn eat_framebuffer(&mut self, buf: &[u16]) -> Result<(), FbWriteError>;
}

impl FramebufferTarget for EspParallelLCD {
    fn eat_framebuffer(&mut self, buf: &[u16]) -> Result<(), FbWriteError> {
        self.draw_bitmap(
            0,
            0,
            SCREEN_WIDTH as i32,
            SCREEN_HEIGHT as i32,
            buf.as_ptr() as *mut std::ffi::c_void,
        );
        self.restart();
        Ok(())
    }
}
