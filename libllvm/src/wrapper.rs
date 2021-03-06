pub use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMModuleRef, LLVMPassManagerRef,
                        LLVMTypeRef, LLVMValueRef, LLVMBasicBlockRef};
use llvm_sys::execution_engine::{LLVMExecutionEngineRef, LLVMGenericValueRef,
                                 LLVMGenericValueToFloat, LLVMRunFunction};
use llvm_sys::analysis::{LLVMVerifierFailureAction, LLVMVerifyFunction};
pub use llvm_sys::{ LLVMIntPredicate, LLVMRealPredicate };
use llvm_sys::transforms;

use std::ptr;
use libc::{c_char, c_uint, c_ulonglong};
use std::ffi::CString;

pub use llvm_sys::core::*;


pub trait LLVMWrapper<T> {
    fn from_ref(ptr: T) -> Self;
    fn raw_ptr(&self) -> T;
}

macro_rules! make_LLVM_wrapper {
    ($origin:ty, $wrapper:ident) => {
        #[derive(Debug, Clone)]
        pub struct $wrapper($origin);
        impl LLVMWrapper<$origin> for $wrapper {
            fn from_ref(ptr: $origin) -> Self {
                $wrapper(ptr)
            }
            fn raw_ptr(&self) -> $origin {
                self.0.clone()
            }
        }
    };
    ($origin:ty, $wrapper:ident, Copy) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $wrapper($origin);
        impl LLVMWrapper<$origin> for $wrapper {
            fn from_ref(ptr: $origin) -> Self {
                $wrapper(ptr)
            }
            fn raw_ptr(&self) -> $origin {
                self.0.clone()
            }
        }
    }
}

make_LLVM_wrapper!(LLVMModuleRef, LLVMModule);
make_LLVM_wrapper!(LLVMContextRef, LLVMContext);
make_LLVM_wrapper!(LLVMPassManagerRef, LLVMFunctionPassManager);
make_LLVM_wrapper!(LLVMValueRef, LLVMValue, Copy);
make_LLVM_wrapper!(LLVMValueRef, LLVMFunction, Copy);
make_LLVM_wrapper!(LLVMTypeRef, LLVMType, Copy);
make_LLVM_wrapper!(LLVMBuilderRef, LLVMBuilder);
make_LLVM_wrapper!(LLVMBasicBlockRef, LLVMBasicBlock);


pub unsafe fn raw_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap().into_raw()
}


macro_rules! method_type_getter {
    ($name: ident, $fun: ident) => {
        pub fn $name(&self) -> LLVMType {
            unsafe {
                LLVMType::from_ref($fun(self.0))
            }
        }
    };
}

impl LLVMContext {
    pub fn new() -> Self {
        unsafe { LLVMContext(LLVMContextCreate()) }
    }
    method_type_getter!(get_int1_type, LLVMInt1TypeInContext);
    method_type_getter!(get_int8_type, LLVMInt8TypeInContext);
    method_type_getter!(get_int16_type, LLVMInt16TypeInContext);
    method_type_getter!(get_int32_type, LLVMInt32TypeInContext);
    method_type_getter!(get_double_type, LLVMDoubleTypeInContext);
    method_type_getter!(get_void_type, LLVMVoidTypeInContext);

    pub fn get_function_type(ret: &LLVMType, param: &Vec<LLVMType>, is_var_arg: bool) -> LLVMType {
        let mut ps: Vec<_> = param.iter().map(|t| t.raw_ptr()).collect();
        let pc = ps.len() as c_uint;
        let flag = if is_var_arg { 1 } else { 0 };
        let fun = unsafe { LLVMFunctionType(ret.raw_ptr(), ps.as_mut_ptr(), pc, flag) };
        LLVMType::from_ref(fun)
    }

    pub fn get_struct_type(&self, types: &Vec<LLVMType>, packed: bool) -> LLVMType {
        let mut mems: Vec<_> = types.iter().map(|t| t.raw_ptr()).collect();
        let flag = if packed { 1 } else { 0 };
        let t = unsafe {
            LLVMStructTypeInContext(self.0.clone(),
                                    mems.as_mut_ptr(),
                                    mems.len() as c_uint,
                                    flag)
        };
        LLVMType(t)
    }

    pub fn get_const_string(&self, s: &str) -> LLVMValue {
        unsafe {
            LLVMValue::from_ref(LLVMConstStringInContext(self.raw_ptr(),
                                                         raw_string(s),
                                                         s.len() as ::libc::c_uint,
                                                         0))
        }
    }

    pub fn get_double_const(&self, val: f64) -> LLVMValue {
        unsafe {
            LLVMValue::from_ref(LLVMConstReal(self.get_double_type().raw_ptr(),
                                              val as ::libc::c_double))
        }
    }
    pub fn get_int32_const(&self, val: i32) -> LLVMValue {
        unsafe {
            LLVMValue::from_ref(LLVMConstInt(self.get_int32_type().raw_ptr(),
                                             val as c_ulonglong,
                                             1))
        }
    }
    pub fn get_uint8_const(&self, val: u8) -> LLVMValue {
        unsafe {
            LLVMValue::from_ref(LLVMConstInt(self.get_int8_type().raw_ptr(), val as c_ulonglong, 0))
        }
    }
    pub fn get_int1_const(&self, val: u64) -> LLVMValue {
        unsafe {
            LLVMValue::from_ref(LLVMConstInt(self.get_int1_type().raw_ptr(), val as c_ulonglong, 1))
        }
    }

    pub fn append_basic_block(&self, fun: &LLVMFunction, name: &str) -> LLVMBasicBlock {
        unsafe {
            LLVMBasicBlock::from_ref(LLVMAppendBasicBlockInContext(self.raw_ptr(),
                                                                   fun.raw_ptr(),
                                                                   raw_string(name)))
        }
    }
}

impl LLVMModule {
    pub fn new(name: &str) -> Self {
        unsafe { LLVMModule(LLVMModuleCreateWithName(raw_string(name))) }
    }
    pub fn in_ctx(name: &str, ctx: &LLVMContext) -> Self {
        unsafe { LLVMModule(LLVMModuleCreateWithNameInContext(raw_string(name), ctx.raw_ptr())) }
    }
    pub fn dump(&self) {
        unsafe { LLVMDumpModule(self.raw_ptr()) }
    }

    pub fn get_function(&self, fun_name: &str) -> Option<LLVMFunction> {
        unsafe {
            let n = raw_string(fun_name);
            let f = LLVMGetNamedFunction(self.0.clone(), n);
            if f.is_null() {
                None
            } else {
                Some(LLVMFunction::from_ref(f))
            }
        }
    }

    pub fn add_function(&self, fun_name: &str, fty: &LLVMType) -> LLVMFunction {
        unsafe {
            let n = raw_string(fun_name);
            let f = LLVMAddFunction(self.raw_ptr(), raw_string(fun_name), fty.raw_ptr());
            LLVMFunction::from_ref(f)
        }
    }

}

impl LLVMFunctionPassManager {
    pub fn init_for_module(m: &LLVMModule) -> Self {
        unsafe {
            let llfpm = LLVMCreateFunctionPassManagerForModule(m.raw_ptr());
            transforms::scalar::LLVMAddBasicAliasAnalysisPass(llfpm);
            transforms::scalar::LLVMAddInstructionCombiningPass(llfpm);
            transforms::scalar::LLVMAddReassociatePass(llfpm);
            transforms::scalar::LLVMAddGVNPass(llfpm);
            transforms::scalar::LLVMAddCFGSimplificationPass(llfpm);
            // transforms::scalar::LLVMAddDeadStoreEliminationPass(llfpm);
            transforms::scalar::LLVMAddMergedLoadStoreMotionPass(llfpm);
            transforms::scalar::LLVMAddConstantPropagationPass(llfpm);
            transforms::scalar::LLVMAddPromoteMemoryToRegisterPass(llfpm);
            transforms::scalar::LLVMAddTailCallEliminationPass(llfpm);
            LLVMInitializeFunctionPassManager(llfpm);
            LLVMFunctionPassManager(llfpm)
        }
    }
    pub fn run(&self, f: &LLVMFunction) {
        unsafe {
            LLVMRunFunctionPassManager(self.raw_ptr(), f.raw_ptr());
        }
    }
}

impl LLVMType {
    pub fn get_ptr(&self, address_space: usize) -> Self {
        unsafe { LLVMType(LLVMPointerType(self.0.clone(), address_space as c_uint)) }
    }
    pub fn get_element(&self) -> Self {
        unsafe { LLVMType(LLVMGetElementType(self.0.clone())) }
    }

    pub fn get_null_ptr(&self) -> LLVMValue {
        unsafe {
            LLVMValue::from_ref(LLVMConstNull(self.raw_ptr()))
        }
    }
}

impl LLVMValue {
    pub fn set_name(&self, name: &str) {
        unsafe { LLVMSetValueName(self.0.clone(), raw_string(name)) }
    }

    pub fn into_function(self) -> LLVMFunction {
        LLVMFunction::from_ref(self.0)
    }
    pub fn get_type(&self) -> LLVMType {
        unsafe { LLVMType::from_ref(LLVMTypeOf(self.0)) }
    }
    pub fn dump(&self) {
        unsafe {
            LLVMDumpValue(self.raw_ptr())
        }
    }
}

impl LLVMFunction {
    pub fn into_value(self) -> LLVMValue {
        LLVMValue::from_ref(self.0)
    }
    pub fn count_basic_blocks(&self) -> usize {
        unsafe { LLVMCountBasicBlocks(self.raw_ptr()) as usize }
    }

    pub fn count_params(&self) -> usize {
        unsafe { LLVMCountParams(self.raw_ptr()) as usize }
    }

    pub fn get_param(&self, idx: usize) -> LLVMValue {
        unsafe { LLVMValue::from_ref(LLVMGetParam(self.raw_ptr(), idx as c_uint)) }
    }

    pub fn get_entry_basic_block(&self) -> LLVMBasicBlock {
        unsafe { LLVMBasicBlock::from_ref(LLVMGetEntryBasicBlock(self.raw_ptr())) }
    }

    pub fn verify(&self, action: LLVMVerifierFailureAction) -> bool {
        unsafe { LLVMVerifyFunction(self.raw_ptr(), action) == 0 }
    }
}

macro_rules! method_build_instr {
    ($name: ident, $fun: ident, $($param:ident : $ty:ty),* => $t:ident: &str) => {
    pub fn $name(&self, $($param: $ty),* , $t: &str) -> LLVMValue {
        unsafe {
            LLVMValue::from_ref($fun(self.raw_ptr(), $($param.raw_ptr()),*, raw_string($t)))
        }
    }
    };
    ($name: ident, $fun: ident, $($param:ident : $ty:ty),*) => {
    pub fn $name(&self, $($param: $ty),*) -> LLVMValue {
        unsafe {
            LLVMValue::from_ref($fun(self.raw_ptr(), $($param.raw_ptr()),*))
        }
    }
    };
}

impl LLVMBuilder {
    pub fn in_ctx(ctx: &LLVMContext) -> Self {
        unsafe { LLVMBuilder(LLVMCreateBuilderInContext(ctx.raw_ptr())) }
    }

    pub fn set_position(&self, block: &LLVMBasicBlock, instr: &LLVMValue) {
        unsafe { LLVMPositionBuilder(self.raw_ptr(), block.raw_ptr(), instr.raw_ptr()) }
    }
    pub fn set_position_at_end(&self, block: &LLVMBasicBlock) {
        unsafe { LLVMPositionBuilderAtEnd(self.raw_ptr(), block.raw_ptr()) }
    }

    pub fn get_insert_block(&self) -> LLVMBasicBlock {
        unsafe { LLVMBasicBlock::from_ref(LLVMGetInsertBlock(self.raw_ptr())) }
    }

    method_build_instr!(alloca, LLVMBuildAlloca, ty: &LLVMType => dest: &str);
    method_build_instr!(phi, LLVMBuildPhi, ty: &LLVMType => dest: &str);
    method_build_instr!(load, LLVMBuildLoad, ptr: &LLVMValue => dest: &str);
    method_build_instr!(store, LLVMBuildStore, val: &LLVMValue, ptr: &LLVMValue);
    method_build_instr!(ret, LLVMBuildRet, val: &LLVMValue);
    method_build_instr!(cond_br, LLVMBuildCondBr, cond: &LLVMValue, then: &LLVMBasicBlock, el: &LLVMBasicBlock);
    method_build_instr!(br, LLVMBuildBr, cont: &LLVMBasicBlock);
    method_build_instr!(bit_cast, LLVMBuildBitCast, val: &LLVMValue, dest_ty: &LLVMType => dest: &str);

    pub fn phi_node<'a, I>(&self, ty: &LLVMType, incoming: I, dest: &str) -> LLVMValue
        where I: IntoIterator<Item=&'a (&'a LLVMValue, &'a LLVMBasicBlock)>
    {
        unsafe {
            let phi = LLVMBuildPhi(self.raw_ptr(), ty.raw_ptr(), raw_string(dest));
            let (mut vals, mut blks): (Vec<_>, Vec<_>) = incoming
                .into_iter()
                .map(|&(val, blk)| (val.raw_ptr(), blk.raw_ptr()))
                .unzip();
            let count = vals.len();
            LLVMAddIncoming(phi, vals.as_mut_ptr(), blks.as_mut_ptr(), count as c_uint);
            LLVMValue::from_ref(phi)
        }
    }

    pub fn ret_void(&self) -> LLVMValue {
        unsafe {
            LLVMValue::from_ref(LLVMBuildRet(self.raw_ptr(), ptr::null_mut()))
        }
    }
    pub fn call(&self, fun: &LLVMFunction, args: &mut Vec<LLVMValue>, name: &str) -> LLVMValue {
        let mut _args: Vec<_> = args.iter_mut().map(|arg| arg.raw_ptr()).collect();
        unsafe {
            let f = fun.raw_ptr();
            let ret = LLVMBuildCall(self.raw_ptr(),
                                    f,
                                    _args.as_mut_ptr(),
                                    args.len() as c_uint,
                                    raw_string(name));
            LLVMValue::from_ref(ret)
        }

    }

    pub fn struct_field_ptr(&self, ptr: &LLVMValue, idx: usize, name: &str) -> LLVMValue {
        unsafe {
            let ret =
                LLVMBuildStructGEP(self.raw_ptr(), ptr.raw_ptr(), idx as u32, raw_string(name));
            LLVMValue::from_ref(ret)
        }
    }
}

impl LLVMBasicBlock {
    pub fn get_parent(&self) -> LLVMFunction {
        unsafe { LLVMFunction::from_ref(LLVMGetBasicBlockParent(self.raw_ptr())) }
    }

    pub fn get_first_instr(&self) -> LLVMValue {
        unsafe { LLVMValue::from_ref(LLVMGetFirstInstruction(self.raw_ptr())) }
    }
}

impl Drop for LLVMModule {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeModule(self.0);
        }
    }
}
impl Drop for LLVMContext {
    fn drop(&mut self) {
        unsafe {
            LLVMContextDispose(self.0);
        }
    }
}
impl Drop for LLVMBuilder {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeBuilder(self.0);
        }
    }
}
impl Drop for LLVMFunctionPassManager {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposePassManager(self.0);
        }
    }
}

pub type LLVMOperateBuild = unsafe extern "C" fn(LLVMBuilderRef,
                                                 LLVMValueRef,
                                                 LLVMValueRef,
                                                 *const ::libc::c_char)
                                                 -> LLVMValueRef;
