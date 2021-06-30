use memmap::*;
use std::fs::File;
use std::convert::TryInto;
use format::*;
pub mod format;

pub fn mmap_from_file(filepath : &str) -> Result<Mmap,std::io::Error> {
    let file = File::open(filepath)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    return Ok(mmap);
}

pub fn read_xwd_file_header(mmap : &Mmap) -> XwdFileHeader {
    XwdFileHeader {
        header_size      : u32::from_be_bytes((&mmap[0..4]).try_into().unwrap()),
        file_version     : u32::from_be_bytes((&mmap[4..8]).try_into().unwrap()),
   	    pixmap_format    : u32::from_be_bytes((&mmap[8..12]).try_into().unwrap()),
	    pixmap_depth     : u32::from_be_bytes((&mmap[12..16]).try_into().unwrap()),
	    pixmap_width     : u32::from_be_bytes((&mmap[16..20]).try_into().unwrap()),
	    pixmap_height    : u32::from_be_bytes((&mmap[20..24]).try_into().unwrap()),
	    xoffset          : u32::from_be_bytes((&mmap[24..28]).try_into().unwrap()),
	    byte_order       : u32::from_be_bytes((&mmap[28..32]).try_into().unwrap()), 
	    bitmap_unit      : u32::from_be_bytes((&mmap[32..36]).try_into().unwrap()),		
	    bitmap_bit_order : u32::from_be_bytes((&mmap[36..40]).try_into().unwrap()),
	    bitmap_pad       : u32::from_be_bytes((&mmap[40..44]).try_into().unwrap()),
	    bits_per_pixel   : u32::from_be_bytes((&mmap[44..48]).try_into().unwrap()),
	    bytes_per_line   : u32::from_be_bytes((&mmap[48..52]).try_into().unwrap()),
	    visual_class     : u32::from_be_bytes((&mmap[52..56]).try_into().unwrap()),
	    red_mask         : u32::from_be_bytes((&mmap[56..60]).try_into().unwrap()),
	    green_mask       : u32::from_be_bytes((&mmap[60..64]).try_into().unwrap()),
	    blue_mask        : u32::from_be_bytes((&mmap[64..68]).try_into().unwrap()),
	    bits_per_rgb     : u32::from_be_bytes((&mmap[68..72]).try_into().unwrap()),
	    colormap_entries : u32::from_be_bytes((&mmap[72..76]).try_into().unwrap()),
	    ncolors          : u32::from_be_bytes((&mmap[76..80]).try_into().unwrap()),
	    window_width     : u32::from_be_bytes((&mmap[80..84]).try_into().unwrap()),
	    window_height    : u32::from_be_bytes((&mmap[84..88]).try_into().unwrap()),
	    window_x         : u32::from_be_bytes((&mmap[88..92]).try_into().unwrap()),
	    window_y         : u32::from_be_bytes((&mmap[92..96]).try_into().unwrap()),
	    window_bdrwidth  : u32::from_be_bytes((&mmap[96..100]).try_into().unwrap())
    }
}

pub fn read_xwd_colors(mmap : &Mmap, header : &XwdFileHeader) -> Vec<XwdColor> {
    let mut colors_read = 0;
    let mut xwd_colors = Vec::new();
    for chunk in mmap.split_at(header.header_size as usize).1.chunks(XWD_COLOR_SIZE) {
        if colors_read >= header.ncolors {
            break; 
        }
        xwd_colors.push(XwdColor {
            pixel : u32::from_be_bytes((chunk[0..4]).try_into().unwrap()),
            red   : u16::from_be_bytes((chunk[4..6]).try_into().unwrap()),
            green : u16::from_be_bytes((chunk[6..8]).try_into().unwrap()),
            blue  : u16::from_be_bytes((chunk[8..10]).try_into().unwrap()),
            flags : chunk[10],
            pad   : chunk[11]
        });
        colors_read = colors_read + 1;
    }
    return xwd_colors;
}

pub fn read_window_name(mmap : &Mmap, header : &XwdFileHeader) -> String {
    let string_slice = &mmap[XWD_HEADER_SIZE..(header.header_size as usize)];
    return std::str::from_utf8(string_slice).unwrap().to_string();
}

pub fn raw_image_data<'a>(mmap : &'a Mmap, header : &XwdFileHeader) -> &'a [u8] {
    mmap.split_at(mmap.len() - ((header.window_height * header.bytes_per_line) + 1 ) as usize).1
}

pub struct LineScanner<'a> {
    header       : &'a XwdFileHeader,
    raw_pixels   : &'a [u8],
    current_line : usize
}

impl<'a> Iterator for LineScanner<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<&'a [u8]> { 
        let w = self.header.bytes_per_line as usize;
        let start = self.current_line * w;
        let end = (self.current_line  * w) + w; 
        if self.current_line < self.header.window_height as usize {
            self.current_line = self.current_line + 1; 
            return Some(&self.raw_pixels[start..end]);
        } else {
            return None;
        }
    }
}

pub fn line_scanner<'a>(header       : &'a XwdFileHeader,
                        raw_pixels   : &'a [u8]
                        ) -> LineScanner<'a> {
    LineScanner {
        header     : header,
        raw_pixels : raw_pixels,
        current_line : 0
    }
}

pub struct SubScanner<'a> {
    header       : &'a XwdFileHeader,
    raw_pixels   : &'a [u8],
    x            : usize,
    y            : usize,
    width        : usize,
    height       : usize,
    current_line : usize
}

impl<'a> Iterator for SubScanner<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<&'a [u8]> { 
        let bpp = self.header.bits_per_pixel/8;
        let w = (self.header.window_width * bpp) as usize;
        let start = (self.current_line + self.y) * w + self.x * (bpp as usize);
        let end = ((self.current_line + self.y) * w) + (self.x + self.width) * (bpp as usize); 
        if self.current_line < self.height {
            self.current_line = self.current_line + 1; 
            return Some(&self.raw_pixels[start..end]);
        } else {
            return None;
        }
    }
}

pub fn sub_scanner<'a>(header     : &'a XwdFileHeader,
                       raw_pixels : &'a [u8],
                       x          : usize,
                       y          : usize,
                       width      : usize,
                       height     : usize) -> SubScanner<'a> {
    if x > header.window_width as usize {
        panic!("sub_scanner: x too large");
    }
    if y > header.window_height as usize {
        panic!("sub_scanner: y too large");
    }
    if x + width > header.window_width as usize {
        panic!("sub_scanner: width too large");
    }
    if y + height > header.window_height as usize {
        panic!("sub_scanner: height too large");
    }
    SubScanner {
        header       : header,
        raw_pixels   : raw_pixels,
        x            : x,
        y            : y,
        width        : width,
        height       : height,
        current_line : 0
    }
}

fn shift_offset(n : u32) -> u32 {
    let mut m = n;
    for i in 0..32 {
        if (m & 1) == 1 {
            return i
        }
        m = m >> 1;
    }
    return 0;
}

pub fn copy_into_rgb888_vec(xwdf : &XwdFileHeader, raw_data : &[u8]) -> Vec<(u8,u8,u8)> {
    match xwdf.bits_per_pixel {
        8 => {
            raw_data.chunks(1).map(|chunk| (chunk[0],chunk[0],chunk[0]) ).collect()
        },
        16 => {
            let rshift = shift_offset(xwdf.red_mask);
            let gshift = shift_offset(xwdf.green_mask);
            let bshift = shift_offset(xwdf.blue_mask);
            raw_data.chunks(2).map(|chunk| {
                    let pixeldata = u32::from_be_bytes(([0,0,chunk[0],chunk[1]]).try_into().unwrap());
                    let red   = (((pixeldata & xwdf.red_mask) >> rshift) << 3).to_be_bytes()[3];
                    let green = (((pixeldata & xwdf.green_mask) >> gshift) << 2).to_be_bytes()[3];
                    let blue  = (((pixeldata & xwdf.blue_mask) >> bshift) << 3).to_be_bytes()[3];   
                    return (red,green,blue);
            }).collect()
        },
        32 => {
            raw_data.chunks(4).map(|chunk| { return (chunk[3],chunk[2],chunk[1]);}).collect()
        }
        bits => panic!("copy_into_rgb888_vec: unsupported bit depth {}", bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memmap_works() {
        let mmap = mmap_from_file("Cargo.toml").unwrap(); 
        assert_eq!(b"[package", &mmap[0..8]);
    }
    #[test]
    fn scanners_agree() {
        let mmap = mmap_from_file("./test/Xvfb_screen24").unwrap();
        let header = read_xwd_file_header(&mmap); 
        let rawd = raw_image_data(&mmap,&header);
        let ls = line_scanner(&header,&rawd);
        let mut ss = sub_scanner(&header,&rawd,0,0,header.window_width as usize,header.window_height as usize);
        for l in ls {
            assert_eq!(l,ss.next().unwrap());
        }
        assert_eq!(None,ss.next());

        let mut ls = line_scanner(&header,&rawd);
        let ss = sub_scanner(&header,&rawd,0,0,header.window_width as usize,header.window_height as usize);
        let mut count = 0;
        for s in ss {
            count = count + 1;
            assert_eq!(s,ls.next().unwrap());
        }
        assert_eq!(None,ls.next());
        assert_eq!(count,header.window_height);
    }
    #[test]
    fn sub_scanner_test() {
        let mmap = mmap_from_file("./test/Xvfb_screen24").unwrap();
        let header = read_xwd_file_header(&mmap); 
        let rawd = raw_image_data(&mmap,&header);
        let mut ls = line_scanner(&header,&rawd);
        let ss = sub_scanner(&header,&rawd,0,0,20,20);
        for l in ss {            
            let li = ls.next().unwrap();
            let mut indx = 0;
            for u in l {
                assert_eq!(*u,li[indx]);
                indx = indx + 1;
            }
        }

        let mut ls = line_scanner(&header,&rawd);
        let ss = sub_scanner(&header,&rawd,20,0,20,20);
        for l in ss {            
            let li = ls.next().unwrap();
            assert_eq!(*l,li[80..160]);
        }

        let mut ls = line_scanner(&header,&rawd);
        let ss = sub_scanner(&header,&rawd,20,20,20,20);
        for _ in 0..20 {
            ls.next().unwrap();
        }
        for l in ss {            
            let li = ls.next().unwrap();
            assert_eq!(*l,li[80..160]);
        }
    }
    #[test]
    fn test_shift_offset() {
        assert_eq!(shift_offset(16711680),16);
        assert_eq!(shift_offset(63488),11);
        assert_eq!(shift_offset(2016),5);
        assert_eq!(shift_offset(31),0);
    }
}
