use std::{
    fmt::Debug,
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
};

use tracing::debug;

use crate::{
    architecture::arm::{
        ArmCommunicationInterface, ArmError, RawDapAccess, RegisterAddress, SwoAccess,
        communication_interface::{DapProbe, SwdSequence, UninitializedArmProbe},
    },
    probe::{DebugProbe, DebugProbeError, WireProtocol, DebugProbeInfo, DebugProbeSelector, ProbeCreationError, ProbeFactory},
};

mod commands;
use commands::*;

#[derive(Debug)]
struct TCPProbe {
    sock: TcpStream,
}

impl TCPProbe {
    fn connect(addr: impl ToSocketAddrs) -> Result<Self, ProbeCreationError>{
        let sock = TcpStream::connect(addr).map_err(|_| ProbeCreationError::CouldNotOpen)?;
        Ok(Self {
            sock: sock
        })
    }

    fn send_message(&mut self, msg: &[u8]) -> Result<(), DebugProbeError> {
        let size = msg
            .len()
            .try_into()
            .map_err(|_| DebugProbeError::Other("Command too big".into()))?;
        self.sock
            .write_all(&[size])
            .map_err(|_| DebugProbeError::Other("Could not send to probe".into()))?;
        self.sock
            .write_all(&msg)
            .map_err(|_| DebugProbeError::Other("Could not send to probe".into()))?;
        Ok(())
    }

    fn recv_message(&mut self) -> Result<Vec<u8>, DebugProbeError> {
        let mut size = [0u8; 1];
        self.sock
            .read_exact(&mut size)
            .map_err(|_| DebugProbeError::Other("Could not recv from probe".into()))?;
        let mut msg = vec![0u8; size[0] as usize];
        self.sock
            .read_exact(&mut msg)
            .map_err(|_| DebugProbeError::Other("Could not read from probe".into()))?;
        Ok(msg)
    }

    fn do_command<Cmd: Command>(&mut self, cmd: Cmd) -> Result<Cmd::Reply, DebugProbeError> {
        self.send_message(&cmd.serialize()?)?;
        Cmd::Reply::deserialize(&self.recv_message()?)
    }
}

impl SwoAccess for TCPProbe {
    fn enable_swo(&mut self, _config: &crate::architecture::arm::SwoConfig) -> Result<(), ArmError> {
        Err(ArmError::NotImplemented("swo"))
    }

    fn disable_swo(&mut self) -> Result<(), ArmError> {
        Err(ArmError::NotImplemented("swo"))
    }

    fn read_swo_timeout(&mut self, _timeout: std::time::Duration) -> Result<Vec<u8>, ArmError> {
        Err(ArmError::NotImplemented("swo"))
    }
}

impl RawDapAccess for TCPProbe {
    fn raw_read_register(&mut self, address: RegisterAddress) -> Result<u32, ArmError> {
        Ok(match address {
            RegisterAddress::DpRegister(dp_reg) => self.do_command(ReadRawDp(dp_reg.address))?.0,
            RegisterAddress::ApRegister(ap_reg) => self.do_command(ReadRawAp(ap_reg))?.0,
        })
    }

    fn raw_write_register(&mut self, address: RegisterAddress, value: u32) -> Result<(), ArmError> {
        Ok(match address {
            RegisterAddress::DpRegister(dp_reg) => {
                self.do_command(WriteRawDp(dp_reg.address, value))?.0
            }
            RegisterAddress::ApRegister(ap_reg) => self.do_command(WriteRawAp(ap_reg, value))?.0,
        })
    }

    fn jtag_sequence(&mut self, _cycles: u8, _tms: bool, _tdi: u64) -> Result<(), DebugProbeError> {
        Err(DebugProbeError::InterfaceNotAvailable {
            interface_name: "jtag",
        })
    }

    fn swj_sequence(&mut self, bit_len: u8, bits: u64) -> Result<(), DebugProbeError> {
        SwdSequence::swj_sequence(self, bit_len, bits)
    }

    fn swj_pins(
        &mut self,
        pin_out: u32,
        pin_select: u32,
        pin_wait: u32,
    ) -> Result<u32, DebugProbeError> {
        SwdSequence::swj_pins(self, pin_out, pin_select, pin_wait)
    }

    fn into_probe(self: Box<Self>) -> Box<dyn DebugProbe> {
        self
    }

    fn core_status_notification(
        &mut self,
        state: crate::CoreStatus,
    ) -> Result<(), DebugProbeError> {
        debug!("core status notification: {:?}", state);
        Ok(())
    }
}

impl DapProbe for TCPProbe {}

impl SwdSequence for TCPProbe {
    fn swj_sequence(&mut self, bit_len: u8, bits: u64) -> Result<(), DebugProbeError> {
        debug!("swj_sequence({}, {:#018x})", bit_len, bits);
        Ok(self.do_command(SWJSequence(bit_len, bits))?.0)
    }

    fn swj_pins(
        &mut self,
        _pin_out: u32,
        _pin_select: u32,
        _pin_wait: u32,
    ) -> Result<u32, DebugProbeError> {
        Err(DebugProbeError::NotImplemented { function_name: "swj_pins" })
    }
}

impl DebugProbe for TCPProbe {
    fn get_name(&self) -> &str {
        "TCP Probe"
    }

    fn speed_khz(&self) -> u32 {
        2000
    }

    fn set_speed(&mut self, speed_khz: u32) -> Result<u32, DebugProbeError> {
        Err(DebugProbeError::UnsupportedSpeed(speed_khz))
    }

    fn set_scan_chain(
        &mut self,
        _scan_chain: Vec<probe_rs_target::ScanChainElement>,
    ) -> Result<(), DebugProbeError> {
        Err(DebugProbeError::InterfaceNotAvailable {
            interface_name: "Jtag",
        })
    }

    fn scan_chain(&self) -> Result<&[probe_rs_target::ScanChainElement], DebugProbeError> {
        Err(DebugProbeError::InterfaceNotAvailable {
            interface_name: "Jtag",
        })
    }

    fn attach(&mut self) -> Result<(), DebugProbeError> {
        Ok(())
    }

    fn detach(&mut self) -> Result<(), crate::Error> {
        Ok(())
    }

    fn target_reset(&mut self) -> Result<(), DebugProbeError> {
        Ok(())
    }

    fn target_reset_assert(&mut self) -> Result<(), DebugProbeError> {
        Ok(())
    }

    fn target_reset_deassert(&mut self) -> Result<(), DebugProbeError> {
        Ok(())
    }

    fn select_protocol(&mut self, protocol: WireProtocol) -> Result<(), DebugProbeError> {
        match protocol {
            WireProtocol::Swd => Ok(()),
            WireProtocol::Jtag => Err(DebugProbeError::UnsupportedProtocol(WireProtocol::Jtag)),
        }
    }

    fn active_protocol(&self) -> Option<WireProtocol> {
        Some(WireProtocol::Swd)
    }

    fn into_probe(self: Box<Self>) -> Box<dyn DebugProbe> {
        self
    }

    fn has_arm_interface(&self) -> bool {
        true
    }

    fn try_get_arm_interface<'probe>(
        self: Box<Self>,
    ) -> Result<Box<dyn UninitializedArmProbe + 'probe>, (Box<dyn DebugProbe>, DebugProbeError)>
    {
        debug!("Getting arm interface");
        return Ok(Box::new(ArmCommunicationInterface::new(self, false)));
        // return Ok(self);
    }
}

#[derive(Debug)]
pub(crate) struct TCPProbeFactory;

impl std::fmt::Display for TCPProbeFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TCP Probe factory")
    }
}

impl ProbeFactory for TCPProbeFactory {
    fn open(&self, selector: &DebugProbeSelector) -> Result<Box<dyn DebugProbe>, DebugProbeError> {
        debug!("Opening TCP probe: {selector:?}");
        if let Some(address) = &selector.serial_number {
            Ok(Box::new(TCPProbe::connect(address)?))
        } else {
            Err(ProbeCreationError::NotFound.into())
        }
    }

    fn list_probes(&self) -> Vec<DebugProbeInfo> {
        debug!("Listing TCP probes: 1 found 192.168.1.42:1337");
        vec![DebugProbeInfo {
            identifier: "TCP debug probe".into(),
            vendor_id: 0,
            product_id: 0,
            serial_number: Some("192.168.1.42:1337".into()),
            hid_interface: None,
            probe_factory: &TCPProbeFactory,
        }]
    }
}
