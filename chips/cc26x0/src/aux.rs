//! AUX management
//!
//! NOTE: as of now, the aux controller can only be used by one process at a time.

use kernel::common::VolatileCell;

struct AonWucRegisters {
    _mcu_clk: VolatileCell<u32>,
    aux_clk: VolatileCell<u32>,
    _mcu_cfg: VolatileCell<u32>,
    aux_cfg: VolatileCell<u32>,
    aux_ctl: VolatileCell<u32>,
    pwr_stat: VolatileCell<u32>,
    _shutdown: VolatileCell<u32>,

    _reserved0: VolatileCell<u32>,

    _ctl0: VolatileCell<u32>,
    _ctl1: VolatileCell<u32>,
}

struct AuxWucRegisters {
    mod_clk_en0: VolatileCell<u32>,
    pwr_off_req: VolatileCell<u32>,
    _pwr_dwn_req: VolatileCell<u32>,
    _pwr_dwn_ack: VolatileCell<u32>,

    _clk_lf_req: VolatileCell<u32>,
    _clk_lf_ack: VolatileCell<u32>,

    _res0: [u8; 0x10],

    _wu_evflags: VolatileCell<u32>,
    _wu_evclr: VolatileCell<u32>,

    _adc_clk_ctl: VolatileCell<u32>,
    _tdc_clk_ctl: VolatileCell<u32>,
    _ref_clk_ctl: VolatileCell<u32>,

    _rtc_subsec_inc0: VolatileCell<u32>,
    _rtc_subsec_inc1: VolatileCell<u32>,
    _rtc_subsec_inc_ctl: VolatileCell<u32>,

    mcu_bus_ctl: VolatileCell<u32>,
    _mcu_bus_stat: VolatileCell<u32>,

    _aon_ctl_stat: VolatileCell<u32>,
    _aux_io_latch: VolatileCell<u32>,

    _res1: VolatileCell<u32>,

    _mod_clk_en1: VolatileCell<u32>,
}

pub struct Aux {
    aon_regs: *const AonWucRegisters,
    aux_regs: *const AuxWucRegisters,
}

#[derive(PartialEq)]
pub enum WakeupMode {
    AllowSleep = 0x00,
    WakeUp = 0x01,
}

pub enum AuxClock {
    OscillatorControl = 0x01,
    Semaphores = 0x02,
}

pub const AUX_CTL: Aux = Aux::new();

impl Aux {
    pub const fn new() -> Aux {
        Aux {
            aon_regs: 0x4009_1000 as *const AonWucRegisters,
            aux_regs: 0x400C_6000 as *const AuxWucRegisters,
        }
    }

    pub fn activate_clock(&self, clock: AuxClock) {
        self.power_up();

        let aux_regs: &AuxWucRegisters = unsafe { &*self.aux_regs };
        match clock {
            AuxClock::OscillatorControl => {
                aux_regs.mod_clk_en0.set(aux_regs.mod_clk_en0.get() | 0x40);
            }
            AuxClock::Semaphores => {
                aux_regs.mod_clk_en0.set(aux_regs.mod_clk_en0.get() | 0x1);
            }
        }
    }

    pub fn clock_is_active(&self, clock: AuxClock) -> bool {
        let aux_regs: &AuxWucRegisters = unsafe { &*self.aux_regs };
        match clock {
            AuxClock::OscillatorControl => (aux_regs.mod_clk_en0.get() & 0x40) != 0,
            AuxClock::Semaphores => (aux_regs.mod_clk_en0.get() & 0x1) != 0,
        }
    }

    fn power_up(&self) {
        if self.power_status() == WakeupMode::WakeUp {
            return;
        }

        // Force the AUX to wake up
        self.wakeup_event(WakeupMode::WakeUp);
        // Wait for it to power up
        while self.power_status() != WakeupMode::WakeUp {}
    }

    #[allow(unused)]
    fn power_down(&self) {
        let aon_regs: &AonWucRegisters = unsafe { &*self.aon_regs };
        let aux_regs: &AuxWucRegisters = unsafe { &*self.aux_regs };

        // Disable the clock
        const AUX_CLK_POWER_DOWN_SRC: u32 = 0x00001800;
        aon_regs
            .aux_clk
            .set(aon_regs.aux_clk.get() & !AUX_CLK_POWER_DOWN_SRC);

        // Disable SRAM retention of the aux
        aon_regs.aux_cfg.set(0);

        self.wakeup_event(WakeupMode::AllowSleep);

        // Make a power off request and disconnect the bus
        aux_regs.pwr_off_req.set(1);
        aux_regs.mcu_bus_ctl.set(1);

        while self.power_status() != WakeupMode::AllowSleep {}
    }

    fn wakeup_event(&self, mode: WakeupMode) {
        let aon_regs: &AonWucRegisters = unsafe { &*self.aon_regs };
        match mode {
            WakeupMode::AllowSleep => aon_regs.aux_ctl.set(0),
            WakeupMode::WakeUp => aon_regs.aux_ctl.set(1),
        }
    }

    fn power_status(&self) -> WakeupMode {
        let aon_regs: &AonWucRegisters = unsafe { &*self.aon_regs };

        if (aon_regs.pwr_stat.get() & 0x02) != 0 {
            WakeupMode::WakeUp
        } else {
            WakeupMode::AllowSleep
        }
    }
}
