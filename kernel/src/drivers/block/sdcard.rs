use super::BlockDevice;
use crate::sync::UPSafeCell;
use k210_hal::prelude::*;
use k210_pac::{Peripherals, SPI0};
use k210_soc::{
    fpioa::{self, io},
    gpio, gpiohs,
    sleep::usleep,
    spi::{aitm, frame_format, tmod, work_mode, SPIExt, SPIImpl, SPI},
    sysctl,
};
use lazy_static::lazy_static;

pub struct SDCard<SPI> {
    spi: SPI,
    spi_cs: u32,
    cs_gpionum: u8,
}

// Start Data tokens:
// Tokens (necessary because at nop/idle (and CS active) only 0xff is on the data/command line)
/// Data token start byte, Start Single Block Read
pub const SD_START_DATA_SINGLE_BLOCK_READ: u8 = 0xFE;
/// Data token start byte, Start Multiple Block Read
pub const SD_START_DATA_MULTIPLE_BLOCK_READ: u8 = 0xFE;
/// Data token start byte, Start Single Block Write
pub const SD_START_DATA_SINGLE_BLOCK_WRITE: u8 = 0xFE;
/// Data token start byte, Start Multiple Block Write
pub const SD_START_DATA_MULTIPLE_BLOCK_WRITE: u8 = 0xFC;

pub const SEC_LEN: usize = 512;

/// SD commands
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Cmd {
    /// Software reset
    CMD0 = 0,
    /// Check voltage range (SDC V2)
    CMD8 = 8,
    /// Read CSD register
    CMD9 = 9,
    /// Read CID register
    CMD10 = 10,
    /// Stop to read data
    CMD12 = 12,
    /// Change R/W block size
    CMD16 = 16,
    /// Read block
    CMD17 = 17,
    /// Read multiple blocks
    CMD18 = 18,
    /// Number of blocks to erase (SDC)
    ACMD23 = 23,
    /// Write a block
    CMD24 = 24,
    /// Write multiple blocks
    CMD25 = 25,
    /// Initiate initialization process (SDC)
    ACMD41 = 41,
    /// Leading command for ACMD
    CMD55 = 55,
    /// Read OCR
    CMD58 = 58,
    /// Enable/disable CRC check
    CMD59 = 59,
}

#[derive(Debug, Copy, Clone)]
pub enum InitError {
    CMDFailed(Cmd, u8),
    CardCapacityStatusNotSet([u8; 4]),
    CannotGetCardInfo,
}

/// Card Specific Data: CSD Register
#[derive(Debug, Copy, Clone)]
pub struct SDCardCSD {
    pub csd_struct: u8,             // CSD structure
    pub sys_spec_version: u8,       // System specification version
    pub reserved1: u8,              // Reserved
    pub taac: u8,                   // Data read access-time 1
    pub nsac: u8,                   // Data read access-time 2 in CLK cycles
    pub max_bus_clk_frec: u8,       // Max. bus clock frequency
    pub card_comd_classes: u16,     // Card command classes
    pub rd_block_len: u8,           // Max. read data block length
    pub part_block_read: u8,        // Partial blocks for read allowed
    pub wr_block_misalign: u8,      // Write block misalignment
    pub rd_block_misalign: u8,      // Read block misalignment
    pub dsr_impl: u8,               // DSR implemented
    pub reserved2: u8,              // Reserved
    pub device_size: u32,           // Device Size
    pub erase_gr_size: u8,          // Erase group size
    pub erase_gr_mul: u8,           // Erase group size multiplier
    pub wr_protect_gr_size: u8,     // Write protect group size
    pub wr_protect_gr_enable: u8,   // Write protect group enable
    pub man_defl_ecc: u8,           // Manufacturer default ECC
    pub wr_speed_fact: u8,          // Write speed factor
    pub max_wr_block_len: u8,       // Max. write data block length
    pub write_block_pa_partial: u8, // Partial blocks for write allowed
    pub reserved3: u8,              // Reserved
    pub content_protect_appli: u8,  // Content protection application
    pub file_format_group: u8,      // File format group
    pub copy_flag: u8,              // Copy flag (OTP)
    pub perm_wr_protect: u8,        // Permanent write protection
    pub temp_wr_protect: u8,        // Temporary write protection
    pub file_format: u8,            // File Format
    pub ecc: u8,                    // ECC code
    pub csd_crc: u8,                // CSD CRC
    pub reserved4: u8,              // always 1
}

#[derive(Debug, Copy, Clone)]
pub struct SDCardCID {
    pub manufacturer_id: u8, // Manufacturer ID
    pub oem_appli_id: u16,   // OEM/Application ID
    pub prod_name1: u32,     // Product Name part 1
    pub prod_name2: u8,      // Product Name part 2
    pub prod_rev: u8,        // Product Revision
    pub prod_sn: u32,        // Product Serial Number
    pub reserved1: u8,       // Reserved1
    pub manufact_date: u16,  // Manufacturing Date
    pub cid_crc: u8,         // CID CRC
    pub reserved2: u8,       // always 1
}

#[derive(Debug, Copy, Clone)]
pub struct SDCardInfo {
    pub sd_csd: SDCardCSD,
    pub sd_cid: SDCardCID,
    pub card_capacity: u64,   // Card Capacity
    pub card_block_size: u64, // Card Block Size
}

impl<X: SPI> SDCard<X> {
    pub fn new(spi: X, spi_cs: u32, cs_gpionum: u8) -> Self {
        Self {
            spi,
            spi_cs,
            cs_gpionum,
        }
    }

    fn cs_hign(&self) {
        gpiohs::set_pin(self.cs_gpionum, true);
    }

    fn cs_low(&self) {
        gpiohs::set_pin(self.cs_gpionum, false);
    }

    fn high_speed_enable(&self) {
        self.spi.set_clk_rate(10_000_000);
    }

    fn lowlevel_init(&self) {
        gpiohs::set_direction(self.cs_gpionum, gpio::direction::OUTPUT);
        self.spi.set_clk_rate(200_000);
    }

    fn write_data(&self, data: &[u8]) {
        self.spi.configure(
            work_mode::MODE0,
            frame_format::STANDARD,
            8, // data bits
            0, // endian
            0, // instruction length
            0, // address length
            0, // wait cycles
            aitm::STANDARD,
            tmod::TRANS,
        );
        self.spi.send_data(self.spi_cs, data);
    }

    fn read_data(&self, data: &mut [u8]) {
        self.spi.configure(
            work_mode::MODE0,
            frame_format::STANDARD,
            8, // data bits
            0, // endian
            0, // instruction length
            0, // address length
            0, // wait cycles
            aitm::STANDARD,
            tmod::RECV,
        );
        self.spi.recv_data(self.spi_cs, data);
    }

    /// Sends a command to the SD card with the specified arguments and CRC.
    ///
    /// # Parameters
    ///
    /// - `cmd`: The command to send.
    /// - `arg`: The argument of the command.
    /// - `crc`: The CRC value for the command.
    fn send_cmd(&self, cmd: Cmd, arg: u32, crc: u8) {
        // SD chip select low
        self.cs_low();
        // Send the Cmd bytes
        self.write_data(&[
            // Construct byte 1
            ((cmd as u8) | 0x40),
            // Construct byte 2
            (arg >> 24) as u8,
            // Construct byte 3
            ((arg >> 16) & 0xff) as u8,
            // Construct byte 4
            ((arg >> 8) & 0xff) as u8,
            // Construct byte 5
            (arg & 0xff) as u8,
            // Construct CRC: byte 6
            crc,
        ]);
    }

    /// Sends end-command sequence to SD card
    fn end_cmd(&self) {
        // SD chip select high
        self.cs_hign();
        // Send the cmd byte
        self.write_data(&[0xff]);
    }

    /// Returns the SD card response.
    ///
    /// # Returns
    ///
    /// - `0xFF` if the sequence failed.
    /// - `0` if the sequence succeeded.
    fn get_response(&self) -> u8 {
        let result = &mut [0u8];
        let mut timeout = 0x0FFF;
        // Check if response is got or a timeout is happen
        while timeout != 0 {
            self.read_data(result);
            // Right response got
            if result[0] != 0xFF {
                return result[0];
            }
            timeout -= 1;
        }
        // After time out
        0xFF
    }

    /// Gets the SD card data response.
    ///
    /// # Returns
    ///
    /// - `0b010`: Data accepted.
    /// - `0b101`: Data rejected due to a CRC error.
    /// - `0b110`: Data rejected due to a write error.
    /// - `0b111`: Data rejected due to another error.
    fn get_dataresponse(&self) -> u8 {
        let response = &mut [0u8];
        // Read resonse
        self.read_data(response);
        // Mask unused bits
        response[0] &= 0x1F;
        if response[0] != 0x05 {
            return 0xFF;
        }
        // Wait null data
        self.read_data(response);
        while response[0] == 0 {
            self.read_data(response);
        }
        // Return response
        0
    }

    /// Reads the CSD card register in SPI mode.
    ///
    /// # Parameters
    ///
    /// - `SD_csd`: Pointer to an SD card CSD register structure.
    ///
    /// # Returns
    ///
    /// - `Err()`: If the sequence failed.
    /// - `Ok(info)`: If the sequence succeeded, returning the info.
    fn get_csdregister(&self) -> Result<SDCardCSD, ()> {
        let mut csd_tab = [0u8; 18];
        // Send CMD9 (CSD register)
        self.send_cmd(Cmd::CMD9, 0, 0);
        // Wait for response in the R1 format (0x00 is no errors)
        if self.get_response() != 0x00 {
            self.end_cmd();
            return Err(());
        }
        if self.get_response() != SD_START_DATA_SINGLE_BLOCK_READ {
            self.end_cmd();
            return Err(());
        }
        // Store CSD register value on csd_tab
        // Get CRC bytes (not really needed by us, but required by SD)
        self.read_data(&mut csd_tab);
        self.end_cmd();
        // see also: https://cdn-shop.adafruit.com/datasheets/TS16GUSDHC6.pdf
        Ok(SDCardCSD {
            // Byte 0
            csd_struct: (csd_tab[0] & 0xC0) >> 6,
            sys_spec_version: (csd_tab[0] & 0x3C) >> 2,
            reserved1: csd_tab[0] & 0x03,
            // Byte 1
            taac: csd_tab[1],
            // Byte 2
            nsac: csd_tab[2],
            // Byte 3
            max_bus_clk_frec: csd_tab[3],
            // Byte 4, 5
            card_comd_classes: (u16::from(csd_tab[4]) << 4) | ((u16::from(csd_tab[5]) & 0xF0) >> 4),
            // Byte 5
            rd_block_len: csd_tab[5] & 0x0F,
            // Byte 6
            part_block_read: (csd_tab[6] & 0x80) >> 7,
            wr_block_misalign: (csd_tab[6] & 0x40) >> 6,
            rd_block_misalign: (csd_tab[6] & 0x20) >> 5,
            dsr_impl: (csd_tab[6] & 0x10) >> 4,
            reserved2: 0,
            // DeviceSize: (csd_tab[6] & 0x03) << 10,
            // Byte 7, 8, 9
            device_size: ((u32::from(csd_tab[7]) & 0x3F) << 16)
                | (u32::from(csd_tab[8]) << 8)
                | u32::from(csd_tab[9]),
            // Byte 10
            erase_gr_size: (csd_tab[10] & 0x40) >> 6,
            // Byte 10, 11
            erase_gr_mul: ((csd_tab[10] & 0x3F) << 1) | ((csd_tab[11] & 0x80) >> 7),
            // Byte 11
            wr_protect_gr_size: (csd_tab[11] & 0x7F),
            // Byte 12
            wr_protect_gr_enable: (csd_tab[12] & 0x80) >> 7,
            man_defl_ecc: (csd_tab[12] & 0x60) >> 5,
            wr_speed_fact: (csd_tab[12] & 0x1C) >> 2,
            // Byte 12,13
            max_wr_block_len: ((csd_tab[12] & 0x03) << 2) | ((csd_tab[13] & 0xC0) >> 6),
            // Byte 13
            write_block_pa_partial: (csd_tab[13] & 0x20) >> 5,
            reserved3: 0,
            content_protect_appli: (csd_tab[13] & 0x01),
            // Byte 14
            file_format_group: (csd_tab[14] & 0x80) >> 7,
            copy_flag: (csd_tab[14] & 0x40) >> 6,
            perm_wr_protect: (csd_tab[14] & 0x20) >> 5,
            temp_wr_protect: (csd_tab[14] & 0x10) >> 4,
            file_format: (csd_tab[14] & 0x0C) >> 2,
            ecc: (csd_tab[14] & 0x03),
            // Byte 15
            csd_crc: (csd_tab[15] & 0xFE) >> 1,
            reserved4: 1,
            // Return the reponse
        })
    }

    /// Reads the CID card register in SPI mode.
    ///
    /// # Parameters
    ///
    /// - `SD_cid`: Pointer to a CID register structure.
    ///
    /// # Returns
    ///
    /// - `Err()`: If the sequence failed.
    /// - `Ok(info)`: If the sequence succeeded, including the info retrieved.
    fn get_cidregister(&self) -> Result<SDCardCID, ()> {
        let mut cid_tab = [0u8; 18];
        // Send CMD10 (CID register)
        self.send_cmd(Cmd::CMD10, 0, 0);
        // Wait for response in the R1 format (0x00 is no errors)
        if self.get_response() != 0x00 {
            self.end_cmd();
            return Err(());
        }
        if self.get_response() != SD_START_DATA_SINGLE_BLOCK_READ {
            self.end_cmd();
            return Err(());
        }
        // Store CID register value on cid_tab
        // Get CRC bytes (not really needed by us, but required by SD)
        self.read_data(&mut cid_tab);
        self.end_cmd();
        Ok(SDCardCID {
            // Byte 0
            manufacturer_id: cid_tab[0],
            // Byte 1, 2
            oem_appli_id: (u16::from(cid_tab[1]) << 8) | u16::from(cid_tab[2]),
            // Byte 3, 4, 5, 6
            prod_name1: (u32::from(cid_tab[3]) << 24)
                | (u32::from(cid_tab[4]) << 16)
                | (u32::from(cid_tab[5]) << 8)
                | u32::from(cid_tab[6]),
            // Byte 7
            prod_name2: cid_tab[7],
            // Byte 8
            prod_rev: cid_tab[8],
            // Byte 9, 10, 11, 12
            prod_sn: (u32::from(cid_tab[9]) << 24)
                | (u32::from(cid_tab[10]) << 16)
                | (u32::from(cid_tab[11]) << 8)
                | u32::from(cid_tab[12]),
            // Byte 13, 14
            reserved1: (cid_tab[13] & 0xF0) >> 4,
            manufact_date: ((u16::from(cid_tab[13]) & 0x0F) << 8) | u16::from(cid_tab[14]),
            // Byte 15
            cid_crc: (cid_tab[15] & 0xFE) >> 1,
            reserved2: 1,
        })
    }

    /// Returns information about a specific SD card.
    ///
    /// # Parameters
    ///
    /// - `cardinfo`: Pointer to an `SD_CardInfo` structure that holds all the information about the SD card.
    ///
    /// # Returns
    ///
    /// - `Err(())`: If the sequence failed.
    /// - `Ok(info)`: If the sequence succeeded, returns the card information.
    fn get_cardinfo(&self) -> Result<SDCardInfo, ()> {
        let mut info = SDCardInfo {
            sd_csd: self.get_csdregister()?,
            sd_cid: self.get_cidregister()?,
            card_capacity: 0,
            card_block_size: 0,
        };
        info.card_block_size = 1 << u64::from(info.sd_csd.rd_block_len);
        info.card_capacity = (u64::from(info.sd_csd.device_size) + 1) * 1024 * info.card_block_size;

        Ok(info)
    }

    /// Initializes the SD card communication in SPI mode.
    ///
    /// # Returns
    ///
    /// Returns `Ok` with SD response info if succeeds, or an `Err` if fails.
    pub fn init(&self) -> Result<SDCardInfo, InitError> {
        // Initialize SD_SPI
        self.lowlevel_init();
        // SD chip select high
        self.cs_hign();
        // NOTE: this reset doesn't always seem to work if the SD access was broken off in the
        // middle of an operation: CMDFailed(CMD0, 127).

        // Send dummy byte 0xFF, 10 times with CS high
        // Rise CS and MOSI for 80 clocks cycles
        // Send dummy byte 0xFF
        self.write_data(&[0xff; 10]);
        // ------------ Put SD in SPI mode --------------
        // SD initialized and set to SPI mode properly

        // Send software reset
        self.send_cmd(Cmd::CMD0, 0, 0x95);
        let result = self.get_response();
        self.end_cmd();
        if result != 0x01 {
            return Err(InitError::CMDFailed(Cmd::CMD0, result));
        }

        // Check voltage range
        self.send_cmd(Cmd::CMD8, 0x01AA, 0x87);
        // 0x01 or 0x05
        let result = self.get_response();
        let mut frame = [0u8; 4];
        self.read_data(&mut frame);
        self.end_cmd();
        if result != 0x01 {
            return Err(InitError::CMDFailed(Cmd::CMD8, result));
        }
        let mut index = 255;
        while index != 0 {
            // <ACMD>
            self.send_cmd(Cmd::CMD55, 0, 0);
            let result = self.get_response();
            self.end_cmd();
            if result != 0x01 {
                return Err(InitError::CMDFailed(Cmd::CMD55, result));
            }
            // Initiate SDC initialization process
            self.send_cmd(Cmd::ACMD41, 0x4000_0000, 0);
            let result = self.get_response();
            self.end_cmd();
            if result == 0x00 {
                break;
            }
            index -= 1;
        }
        if index == 0 {
            return Err(InitError::CMDFailed(Cmd::ACMD41, result));
        }
        index = 255;
        let mut frame = [0u8; 4];
        while index != 0 {
            // Read OCR
            self.send_cmd(Cmd::CMD58, 0, 1);
            let result = self.get_response();
            self.read_data(&mut frame);
            self.end_cmd();
            if result == 0 {
                break;
            }
            index -= 1;
        }
        if index == 0 {
            return Err(InitError::CMDFailed(Cmd::CMD58, result));
        }
        if (frame[0] & 0x40) == 0 {
            return Err(InitError::CardCapacityStatusNotSet(frame));
        }
        self.high_speed_enable();
        self.get_cardinfo()
            .map_err(|()| InitError::CannotGetCardInfo)
    }

    /// Reads a block of data from the SD card.
    ///
    /// # Parameters
    ///
    /// - `data_buf`: A slice that receives the data read from the SD card.
    /// - `sector`: The SD card's internal address from which to read.
    ///
    /// # Returns
    ///
    /// - `Err(())`: If the read sequence failed.
    /// - `Ok(())`: If the read sequence succeeded.
    pub fn read_sector(&self, data_buf: &mut [u8], sector: u32) -> Result<(), ()> {
        assert!(data_buf.len() >= SEC_LEN && (data_buf.len() % SEC_LEN) == 0);
        // Send CMD17 to read one block, or CMD18 for multiple
        let flag = if data_buf.len() == SEC_LEN {
            self.send_cmd(Cmd::CMD17, sector, 0);
            false
        } else {
            self.send_cmd(Cmd::CMD18, sector, 0);
            true
        };
        // Check if the SD acknowledged the read block command: R1 response (0x00: no errors)
        if self.get_response() != 0x00 {
            self.end_cmd();
            return Err(());
        }
        let mut error = false;
        let mut tmp_chunk = [0u8; SEC_LEN];
        for chunk in data_buf.chunks_mut(SEC_LEN) {
            if self.get_response() != SD_START_DATA_SINGLE_BLOCK_READ {
                error = true;
                break;
            }
            // Read the SD block data : read NumByteToRead data
            self.read_data(&mut tmp_chunk);
            for (a, b) in chunk.iter_mut().zip(tmp_chunk.iter()) {
                *a = *b;
            }
            // Get CRC bytes (not really needed by us, but required by SD)
            let mut frame = [0u8; 2];
            self.read_data(&mut frame);
        }
        self.end_cmd();
        if flag {
            self.send_cmd(Cmd::CMD12, 0, 0);
            self.get_response();
            self.end_cmd();
            self.end_cmd();
        }
        // It is an error if not everything requested was read
        if error {
            Err(())
        } else {
            Ok(())
        }
    }

    /// Writes a block of data to the SD card.
    ///
    /// # Parameters
    ///
    /// - `data_buf`: A slice containing the data to be written to the SD card.
    /// - `sector`: The address on the SD card where the data will be written.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating whether the write operation was successful:
    /// - `Err(())`: If the write sequence failed.
    /// - `Ok(())`: If the write sequence succeeded.
    pub fn write_sector(&self, data_buf: &[u8], sector: u32) -> Result<(), ()> {
        assert!(data_buf.len() >= SEC_LEN && (data_buf.len() % SEC_LEN) == 0);
        let mut frame = [0xff, 0x00];
        if data_buf.len() == SEC_LEN {
            frame[1] = SD_START_DATA_SINGLE_BLOCK_WRITE;
            self.send_cmd(Cmd::CMD24, sector, 0);
        } else {
            frame[1] = SD_START_DATA_MULTIPLE_BLOCK_WRITE;
            self.send_cmd(
                Cmd::ACMD23,
                (data_buf.len() / SEC_LEN).try_into().unwrap(),
                0,
            );
            self.get_response();
            self.end_cmd();
            self.send_cmd(Cmd::CMD25, sector, 0);
        }
        // Check if the SD acknowledged the write block command: R1 response (0x00: no errors)
        if self.get_response() != 0x00 {
            self.end_cmd();
            return Err(());
        }
        let mut tmp_chunk = [0u8; SEC_LEN];
        for chunk in data_buf.chunks(SEC_LEN) {
            // Send the data token to signify the start of the data
            self.write_data(&frame);
            // Write the block data to SD : write count data by block
            for (a, &b) in tmp_chunk.iter_mut().zip(chunk.iter()) {
                *a = b;
            }
            self.write_data(&tmp_chunk);
            // Put dummy CRC bytes
            self.write_data(&[0xff, 0xff]);
            // Read data response
            if self.get_dataresponse() != 0x00 {
                self.end_cmd();
                return Err(());
            }
        }
        self.end_cmd();
        self.end_cmd();
        Ok(())
    }
}

// GPIOHS GPIO number to use for controlling the SD card CS pin
const SD_CS_GPIONUM: u8 = 7;
// CS value passed to SPI controller, this is a dummy value as `SPI0_CS3` is not mapping to anything in the FPIOA
const SD_CS: u32 = 3;

// Connect pins to internal functions
fn io_init() {
    fpioa::set_function(io::SPI0_SCLK, fpioa::function::SPI0_SCLK);
    fpioa::set_function(io::SPI0_MOSI, fpioa::function::SPI0_D0);
    fpioa::set_function(io::SPI0_MISO, fpioa::function::SPI0_D1);
    fpioa::set_function(io::SPI0_CS0, fpioa::function::gpiohs(SD_CS_GPIONUM));
    fpioa::set_io_pull(io::SPI0_CS0, fpioa::pull::DOWN); // GPIO output=pull down
}

lazy_static! {
    static ref PERIPHERALS: UPSafeCell<Peripherals> =
        unsafe { UPSafeCell::new(Peripherals::take().unwrap()) };
}

fn init() -> SDCard<SPIImpl<SPI0>> {
    // wait previous output
    usleep(100_000);
    let peripherals = unsafe { Peripherals::steal() };
    sysctl::pll_set_freq(sysctl::pll::PLL0, 800_000_000).unwrap();
    sysctl::pll_set_freq(sysctl::pll::PLL1, 300_000_000).unwrap();
    sysctl::pll_set_freq(sysctl::pll::PLL2, 45_158_400).unwrap();
    let clocks = k210_hal::clock::Clocks::new();
    peripherals.UARTHS.configure(115_200.bps(), &clocks);
    io_init();

    let spi = peripherals.SPI0.constrain();
    let sd = SDCard::new(spi, SD_CS, SD_CS_GPIONUM);
    let info = sd.init().unwrap();
    let num_sectors = info.card_capacity / 512;
    assert!(num_sectors > 0);

    sd
}

pub struct SDCardWrapper(UPSafeCell<SDCard<SPIImpl<SPI0>>>);

impl SDCardWrapper {
    pub fn new() -> Self {
        unsafe { Self(UPSafeCell::new(init())) }
    }
}

impl BlockDevice for SDCardWrapper {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_sector(buf, block_id as u32)
            .unwrap();
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_sector(buf, block_id as u32)
            .unwrap();
    }
}
