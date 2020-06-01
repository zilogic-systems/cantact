//! This crate provides a userspace driver for the CANtact family of
//! Controller Area Network (CAN) devices.
//!
//! The rust library provided by this crate can be used directly to build
//! applications for CANtact. The crate also provides bindings for other
//! langauges.
//!
//! Internally, this crate uses the [rusb](https://github.com/a1ien/rusb)
//! library to communicate with the device via libusb. This works on
//! Linux, Mac, and Windows systems with libusb installed. No additional
//! driver installation is required.

#![warn(missing_docs)]

use std::sync::mpsc::{channel, sync_channel, RecvError, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

mod device;
use device::gsusb::*;
use device::*;

pub mod c;
/// Implementation of Python bindings
#[cfg(python)]
pub mod python;

/// Errors generated by this library
#[derive(Debug)]
pub enum Error {
    /// During setup, the device could not be found on the system.
    DeviceNotFound,

    /// Timeout while communicating with the device.
    Timeout,

    /// Attempted to perform an action on a device that is running when this is not allowed.
    Running,

    /// Attempted to perform an action on a device that is not running when this is not allowed.
    NotRunning,

    /// Errors from libusb.
    UsbError,
}

/// Controller Area Network Frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// CAN frame arbitration ID.
    pub can_id: u32,

    /// CAN frame Data Length Code (DLC).
    pub can_dlc: u8,

    /// Device channel used to send or receive the frame.
    pub channel: u8,

    /// Frame data contents.
    pub data: [u8; 8],

    /// Extended (29 bit) arbitration identifier if true,
    /// standard (11 bit) arbitration identifer if false.
    pub ext: bool,

    /// CAN Flexible Data (CAN-FD) frame flag.
    pub fd: bool,

    /// Loopback flag. When true, frame was sent by this device/channel.
    /// False for received frames.
    pub loopback: bool,

    /// Remote Transmission Request (RTR) flag.
    pub rtr: bool,
}
impl Frame {
    // convert to a frame format expected by the device
    fn to_host_frame(&self) -> HostFrame {
        // if frame is extended, set the extended bit in host frame CAN ID
        let mut can_id = if self.ext {
            self.can_id | GSUSB_EXT_FLAG
        } else {
            self.can_id
        };
        // if frame is RTR, set the RTR bit in host frame CAN ID
        can_id = if self.rtr {
            can_id | GSUSB_RTR_FLAG
        } else {
            can_id
        };
        HostFrame {
            echo_id: 1,
            flags: 0,
            reserved: 0,
            can_id: can_id,
            can_dlc: self.can_dlc,
            channel: self.channel,
            data: self.data,
        }
    }
    /// Returns a default CAN frame with all values set to zero/false.
    pub fn default() -> Frame {
        Frame {
            can_id: 0,
            can_dlc: 0,
            data: [0u8; 8],
            channel: 0,
            ext: false,
            fd: false,
            loopback: false,
            rtr: false,
        }
    }
    fn from_host_frame(hf: HostFrame) -> Frame {
        // check the extended bit of host frame
        // if set, frame is extended
        let ext = (hf.can_id & GSUSB_EXT_FLAG) > 0;
        // check the RTR bit of host frame
        // if set, frame is RTR
        let rtr = (hf.can_id & GSUSB_RTR_FLAG) > 0;
        // remove flags from CAN ID
        let can_id = hf.can_id & 0x3FFFFFFF;
        // loopback frame if echo_id is not -1
        let loopback = hf.echo_id != RX_ECHO_ID;
        Frame {
            can_id: can_id,
            can_dlc: hf.can_dlc,
            data: hf.data,
            channel: hf.channel,
            ext: ext,
            fd: false, //TODO
            loopback: loopback,
            rtr: rtr,
        }
    }
}

/// Interface for interacting with CANtact devices
pub struct Interface {
    dev: Device,

    // channel for transmitting can frames to thread for tx
    // when None, thread is not running
    // when this Sender is dropped, the thread is stopped
    can_tx: Option<SyncSender<Frame>>,

    // when true, frames sent by this device are received by the driver
    loopback: bool,
}

// echo id for non-loopback frames
const RX_ECHO_ID: u32 = 4294967295;

impl Interface {
    /// Creates a new interface. This always selects the first device found by
    /// libusb. If no device is found, Error::DeviceNotFound is returned.
    pub fn new() -> Result<Interface, Error> {
        let usb = UsbContext::new();
        let dev = match Device::new(usb) {
            Some(d) => d,
            None => return Err(Error::DeviceNotFound),
        };

        let i = Interface {
            dev: dev,
            can_tx: None,
            loopback: true,
        };

        // TODO get btconsts
        Ok(i)
    }

    /// Start CAN communication on all configured channels.
    /// This function starts a thread to communicate with the device.
    ///
    /// Once started, the device mutex will be locked until the thread
    /// is stopped by calling `Interface.stop`. No changes to device
    /// configuration can be performed while the device is running.
    ///
    /// After starting the device, `Interface.send` can be used to send frames.
    /// For every received frame, the `rx_callback` closure will be called.
    pub fn start(
        &mut self,
        mut rx_callback: impl FnMut(Frame) + Sync + Send + 'static,
    ) -> Result<(), Error> {
        let mode = Mode {
            mode: CanMode::Start as u32,
            flags: 0,
        };
        let loopback = self.loopback.clone();

        // tell the device to go on bus
        // TODO multi-channel
        self.dev.set_mode(0, mode).unwrap();

        let can_rx = self.dev.can_rx_recv.clone();
        // rx callback thread
        thread::spawn(move || loop {
            match can_rx.try_recv() {
                Ok(hf) => rx_callback(Frame::from_host_frame(hf)),
                Err(_) => {}
            }
        });
        Ok(())
    }

    /// Stop CAN communication on all channels.
    pub fn stop(&mut self) -> Result<(), Error> {
        let can_tx = match &self.can_tx {
            Some(v) => v,
            None => return Err(Error::NotRunning),
        };

        // drop the channel to stop the thread
        drop(can_tx);
        // mark thread as not running
        self.can_tx = None;

        let mode = Mode {
            mode: CanMode::Reset as u32,
            flags: 0,
        };

        // TODO multi-channel
        self.dev.set_mode(0, mode).unwrap();

        Ok(())
    }

    /// Set bitrate for specified channel to requested bitrate value in bits per second.
    pub fn set_bitrate(&mut self, channel: u16, bitrate: u32) -> Result<(), Error> {
        match &self.can_tx {
            None => {}
            Some(_) => return Err(Error::Running),
        };

        // TODO get device clock
        let bt = calculate_bit_timing(48000000, bitrate);
        self.dev
            .set_bit_timing(channel, bt)
            .expect("failed to set bit timing");

        Ok(())
    }

    /// Send a CAN frame using the device
    pub fn send(&self, f: Frame) -> Result<(), Error> {
        match &self.can_tx {
            Some(tx) => tx.send(f).unwrap(),
            None => return Err(Error::NotRunning),
        };
        Ok(())
    }
}

fn calculate_bit_timing(device_clk: u32, bitrate: u32) -> BitTiming {
    // use a fixed divider and sampling point
    let brp = 6;
    let sample_point = 0.68;

    let can_clk = device_clk / brp;
    // number of time quanta in segement 1 and segment 2
    // subtract 1 for the fixed sync segment
    let tqs = (can_clk / bitrate) - 1;
    // split tqs into two segments
    let seg1 = (tqs as f32 * sample_point).round() as u32;
    let seg2 = (tqs as f32 * (1.0 - sample_point)).round() as u32;

    BitTiming {
        prop_seg: 0,
        phase_seg1: seg1,
        phase_seg2: seg2,
        sjw: 1,
        brp: brp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bit_timing() {
        let dev_clock = 48000000;
        let bt_1000000 = calculate_bit_timing(dev_clock, 1000000);
        assert_eq!(
            bt_1000000.prop_seg + bt_1000000.phase_seg1 + bt_1000000.phase_seg2 + 1,
            8
        );
        let bt_500000 = calculate_bit_timing(dev_clock, 500000);
        assert_eq!(
            bt_500000.prop_seg + bt_500000.phase_seg1 + bt_500000.phase_seg2 + 1,
            16
        );
        let bt_250000 = calculate_bit_timing(dev_clock, 250000);
        assert_eq!(
            bt_250000.prop_seg + bt_250000.phase_seg1 + bt_250000.phase_seg2 + 1,
            32
        );
        let bt_125000 = calculate_bit_timing(dev_clock, 125000);
        assert_eq!(
            bt_125000.prop_seg + bt_125000.phase_seg1 + bt_125000.phase_seg2 + 1,
            64
        );
        let bt_33000 = calculate_bit_timing(dev_clock, 33000);
    }
}
