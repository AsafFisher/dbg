use alloc::{
    string::{String, ToString},
    vec::Vec,
};

pub const REL_JMP_SIZE: usize = core::mem::size_of::<RelJump>();

#[repr(C, packed)]
pub struct RelJump {
    opcode: u8,
    displacement: u32,
}

fn calc_displacment(from: usize, to: usize, instruction_size: usize) -> Result<u32, String> {
    let displacement = (to as isize).wrapping_sub(from as isize + instruction_size as isize);
    if is_displacement_in_range(displacement) {
        Ok(displacement as u32)
    } else {
        Err("Displacement is too big".to_string())
    }
}
fn is_displacement_in_range(displacement: isize) -> bool {
    (-core::i32::MIN as i64..=core::i32::MAX as i64).contains(&(displacement as i64))
}

pub fn generate_relative_branch(
    from: *const (),
    to: *const (),
    is_call: bool,
) -> Result<Vec<u8>, String> {
    const JMP: u8 = 0xE9;
    const CALL: u8 = 0xE8;
    let displacment = calc_displacment(from as usize, to as usize, core::mem::size_of::<RelJump>());
    match displacment {
        Ok(displacement) => {
            let jmp = RelJump {
                opcode: if is_call { CALL } else { JMP },
                displacement: displacement,
            };
            let jmp: [u8; REL_JMP_SIZE] = unsafe { core::mem::transmute(jmp) };
            Ok(jmp.to_vec())
        }
        Err(err) => Err(err),
    }
}

fn build_mov_rax_i64(immidiate: usize) -> Vec<u8> {
    b"\x48\xb8"
        .iter()
        .chain(immidiate.to_ne_bytes().iter())
        .cloned()
        .collect()
}
fn build_jmp_rax() -> Vec<u8> {
    return b"\xff\xe0".to_vec();
}

pub const ABSOLUTE_BRANCH_SIZE: usize = 12;
pub fn generate_absolute_branch(to: *const ()) -> Result<Vec<u8>, String> {
    Ok(build_mov_rax_i64(to as usize)
        .iter()
        .chain(build_jmp_rax().iter())
        .cloned()
        .collect())
}
