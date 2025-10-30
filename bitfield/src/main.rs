struct Field {
    slots: Vec<Slot>
}

struct Slot {
    number: i8,
    start: i8,
    end: i8,
}

fn main() {
    let bits = [9, 6, 13, 4];
    let mut fields: Vec<Field> = Vec::new();
    // let data = [0u8; 4];
    let mut current_slot = 0;
    let mut slot_postion = 0;
    for field_bits in &bits {
        let mut slots: Vec<Slot> = Vec::new();
        let vacant_slot_bits = 8 - slot_postion;
        let total_slots = (vacant_slot_bits + 7) / 8 + (field_bits - vacant_slot_bits) / 8 + (((field_bits - vacant_slot_bits) % 8) + 7) / 8;
        let mut remaining_bits = field_bits.clone();
        for _ in 0..total_slots {
            let vacancy = 8 - slot_postion;
            let end_position: i8;
            let mut new_slot_position = current_slot.clone();
            if vacancy <= remaining_bits {
                end_position = 7;
                remaining_bits -= vacancy;
                new_slot_position += 1;
            } else {
                end_position = slot_postion + remaining_bits - 1;
                remaining_bits = 0;
            }
            let slot = Slot {
                number: current_slot,
                start: slot_postion,
                end: end_position
            };
            slots.push(slot);
            current_slot = new_slot_position;
            slot_postion = (end_position + 1) % 8;
        }
        fields.push(Field {slots: slots});
    }
    for (i, field) in fields.iter().enumerate() {
        println!("Field {}", i);
        for slot in &field.slots {
            println!("slot number={}, start={}, end={}", slot.number, slot.start, slot.end);
        }
    }
}
