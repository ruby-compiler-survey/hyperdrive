#[cfg(cargo_c)]
use hyperdrive_ruby::rb_control_frame_t;
use hyperdrive_ruby::rb_thread_t;
use hyperdrive_ruby::rb_vm_insn_addr2insn;
use hyperdrive_ruby::VALUE;

use CURRENT_TRACE;
use trace::Trace;

extern "C" {
    #[no_mangle]
    static mut trace_recording: i32;
    #[no_mangle]
    static mut begin_trace: unsafe extern "C" fn(*const rb_thread_t, *const rb_control_frame_t, *const VALUE);
    #[no_mangle]
    static mut record_instruction: unsafe extern "C" fn(*const rb_thread_t, *const rb_control_frame_t, *const VALUE);
}

#[no_mangle]
pub unsafe extern "C" fn hyperdrive_init(){
    begin_trace = hyperdrive_begin_trace;
    record_instruction = hyperdrive_record_instruction;
}

#[no_mangle]
pub unsafe extern "C" fn hyperdrive_record_instruction(
    _thread: *const rb_thread_t,
    _cfp: *const rb_control_frame_t,
    pc: *const VALUE,
) {
    let opcode: i32 = rb_vm_insn_addr2insn(*pc as *const _);
    match &mut CURRENT_TRACE {
        Some(trace) => {
            if *pc == trace.anchor {
                CURRENT_TRACE = None;
                trace_recording = 0;
            } else {
                trace.add_opcode(std::mem::transmute(opcode));
            }
        },
        None => panic!("No trace started"),
    };
}

#[no_mangle]
pub unsafe extern "C" fn hyperdrive_begin_trace(
    _thread: *const rb_thread_t,
    _cfp: *const rb_control_frame_t,
    pc: *const VALUE,
) {
    let trace = Trace {
        opcodes: vec![],
        anchor: *pc,
    };
    match &mut CURRENT_TRACE {
        Some(_) => panic!("trace already recording"),
        _ => {
            trace_recording = 1;
            CURRENT_TRACE = Some(trace);
        },
    };
}

#[no_mangle]
pub unsafe extern "C" fn hyperdrive_stop_recording() {
    trace_recording = 0;
}

#[no_mangle]
pub unsafe extern "C" fn hyperdrive_dump_trace() {
    match &mut CURRENT_TRACE {
        Some(trace) => { println!("trace: {:?}", trace.opcodes) },
        _ => {},
    };
}