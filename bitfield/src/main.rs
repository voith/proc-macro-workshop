struct Field {
    slots: Vec<Slot>,
}

struct Slot {
    number: usize,
    start: u8,
    end: u8,
    get_bit_mask: u8,
    set_width_bit_mask: u64,
}

struct Bitfield<const M: usize, const N: usize> {
    data: [u8; M],
    fields: Vec<Field>,
}

impl<const M: usize, const N: usize> Bitfield<M, N> {
    pub fn new(field_sizes: [u8; N]) -> Self {
        let fields = Self::calculate_layout(&field_sizes);
        Self {
            data: [0u8; M],
            fields,
        }
    }

    fn calculate_layout(field_sizes: &[u8]) -> Vec<Field> {
        let mut fields = Vec::with_capacity(field_sizes.len());
        let mut bit_offset: usize = 0; // total bits placed so far

        for &field_bits in field_sizes {
            let mut remaining: u8 = field_bits;
            let mut slots = Vec::new();

            while remaining > 0 {
                let byte = bit_offset / 8;
                let start = (bit_offset % 8) as u8;
                let vacancy = 8 - start;
                let take = remaining.min(vacancy);
                let end = start + take - 1;
                slots.push(Slot {
                    number: byte,
                    start,
                    end,
                    get_bit_mask: create_get_bit_mask(start, end),
                    set_width_bit_mask: create_set_width_bit_mask(start, end),
                });
                bit_offset += take as usize;
                remaining -= take;
            }

            fields.push(Field { slots });
        }
        fields
    }

    fn set(&mut self, field_index: usize, value: u64) {
        debug_assert!(field_index < self.fields.len());
        let mut num_bits_set: u64 = 0;
        for slot in &self.fields[field_index].slots {
            // clear slot bits incase they were set previously
            self.data[slot.number] &= !slot.get_bit_mask;
            let slot_bits = ((value >> num_bits_set) & slot.set_width_bit_mask) << slot.start;
            self.data[slot.number] |= slot_bits as u8;
            num_bits_set += (slot.end - slot.start + 1) as u64;
        }
    }

    fn get(&self, field_index: usize) -> u64 {
        debug_assert!(field_index < self.fields.len());
        let mut value: u64 = 0b0;
        let mut previous_bits: u64 = 0;
        for slot in &self.fields[field_index].slots {
            value |= ((((self.data[slot.number] & slot.get_bit_mask) as u64) >> slot.start)
                << previous_bits) as u64;
            previous_bits += (slot.end - slot.start + 1) as u64;
        }
        value
    }

    fn set_a(&mut self, value: u64) {
        self.set(0, value);
    }

    fn set_b(&mut self, value: u64) {
        self.set(1, value);
    }

    fn set_c(&mut self, value: u64) {
        self.set(2, value);
    }

    fn set_d(&mut self, value: u64) {
        self.set(3, value);
    }

    fn get_a(&self) -> u16 {
        self.get(0) as u16
    }

    fn get_b(&self) -> u8 {
        self.get(1) as u8
    }

    fn get_c(&self) -> u16 {
        self.get(2) as u16
    }

    fn get_d(&self) -> u8 {
        self.get(3) as u8
    }
}

const fn calculate_data_size(field_sizes: &[u8]) -> usize {
    let mut sum: usize = 0;
    let mut i: usize = 0;
    while i < field_sizes.len() {
        sum += field_sizes[i] as usize;
        i += 1;
    }
    // sum of field_size will always be a multiple of 8
    sum / 8
}

fn create_get_bit_mask(start: u8, end: u8) -> u8 {
    let mut mask: u8 = 0b00000000;
    for i in start..=end {
        mask |= 1 << i;
    }
    mask
}

fn create_set_width_bit_mask(start: u8, end: u8) -> u64 {
    debug_assert!(end >= start && (end - start) < 64);
    let mut mask: u64 = 0b0;
    for i in 0..=end - start {
        mask |= 1 << i;
    }
    mask
}

fn main() {
    const FIELD_SIZES: [u8; 4] = [9, 6, 13, 4];
    const DATA_SIZE: usize = calculate_data_size(&FIELD_SIZES);
    let mut bitfield: Bitfield<DATA_SIZE, { FIELD_SIZES.len() }> = Bitfield::new(FIELD_SIZES);
    let a = 0b1100_0011_1;
    let b = 0b101_010;
    let c = 0x1675;
    let d = 0b1110;

    bitfield.set_a(a);
    bitfield.set_b(b);
    bitfield.set_c(c);
    bitfield.set_d(d);
    println!("a={:016b}, calculated={:016b}", a, bitfield.get_a());
    println!("b={:08b}, calculated={:08b}", b, bitfield.get_b());
    println!("c={:0x}, calculated={:0x}", c, bitfield.get_c());
    println!("d={:08b}, calculated={:08b}", d, bitfield.get_d());
}
