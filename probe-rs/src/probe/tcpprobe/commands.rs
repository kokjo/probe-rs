use crate::probe::DebugProbeError;

pub(crate) trait CommandReply: Sized {
    fn deserialize(data: &[u8]) -> Result<Self, DebugProbeError>;
}

pub(crate) trait Command {
    type Reply: CommandReply;
    fn serialize(&self) -> Result<Vec<u8>, DebugProbeError>;
}

pub(crate) struct ReadStatus(pub u32);

impl CommandReply for ReadStatus {
    fn deserialize(data: &[u8]) -> Result<Self, DebugProbeError> {
        let (&status, data) = data
            .split_first()
            .ok_or_else(|| DebugProbeError::Other("Invalid packet, no status".into()))?;
        if status != 0 {
            return Err(DebugProbeError::Other(format!(
                "Read command error: {}",
                status
            )));
        }
        Ok(Self(u32::from_be_bytes(data.try_into().map_err(|_| {
            DebugProbeError::Other("Invalid packet length".into())
        })?)))
    }
}

pub(crate) struct WriteStatus(pub ());

impl CommandReply for WriteStatus {
    fn deserialize(data: &[u8]) -> Result<Self, DebugProbeError> {
        let (&status, _data) = data
            .split_first()
            .ok_or_else(|| DebugProbeError::Other("Invalid packet, no status".into()))?;
        if status != 0 {
            return Err(DebugProbeError::Other(format!(
                "Write command error: {}",
                status
            )));
        }
        Ok(WriteStatus(()))
    }
}

pub(crate) struct ReadRawDp(pub u8);

impl Command for ReadRawDp {
    type Reply = ReadStatus;

    fn serialize(&self) -> Result<Vec<u8>, DebugProbeError> {
        Ok(vec![0x00, self.0])
    }
}

pub(crate) struct WriteRawDp(pub u8, pub u32);

impl Command for WriteRawDp {
    type Reply = WriteStatus;

    fn serialize(&self) -> Result<Vec<u8>, DebugProbeError> {
        let mut msg = vec![0x01, self.0];
        msg.extend(self.1.to_be_bytes());
        Ok(msg)
    }
}

pub(crate) struct ReadRawAp(pub u8);

impl Command for ReadRawAp {
    type Reply = ReadStatus;

    fn serialize(&self) -> Result<Vec<u8>, DebugProbeError> {
        Ok(vec![0x02, self.0])
    }
}

pub(crate) struct WriteRawAp(pub u8, pub u32);

impl Command for WriteRawAp {
    type Reply = WriteStatus;

    fn serialize(&self) -> Result<Vec<u8>, DebugProbeError> {
        let mut msg = vec![0x03, self.0];
        msg.extend(self.1.to_be_bytes());
        Ok(msg)
    }
}

pub(crate) struct SWJSequence(pub u8, pub u64);

impl Command for SWJSequence {
    type Reply = WriteStatus;

    fn serialize(&self) -> Result<Vec<u8>, DebugProbeError> {
        let mut msg = vec![0x04, self.0];
        msg.extend(self.1.to_be_bytes());
        Ok(msg)
    }
}