/*
 * BoltFFI Ruby runtime. Generated extensions include this file.
 *
 * Every value crosses the boundary as a plain Ruby object. The helpers here
 * follow these rules:
 *
 * 1. Convert and check every Ruby argument before Rust runs. A Ruby exception
 *    is a longjmp, so C code must not own memory that a raise would skip.
 */
#ifndef BOLTFFI_RUBY_H
#define BOLTFFI_RUBY_H

#include <float.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>

#include <ruby.h>

#include "boltffi.h"

/* ---- errors ------------------------------------------------------------ */

NORETURN(static void boltffi_ruby_wrong_type(VALUE value, const char *expected));
static void boltffi_ruby_wrong_type(VALUE value, const char *expected) {
    rb_raise(rb_eTypeError, "wrong argument type %" PRIsVALUE " (expected %s)", rb_obj_class(value), expected);
}

/* ---- Ruby to C --------------------------------------------------------- */

static inline bool boltffi_ruby_negative(VALUE value) {
    if (RB_FIXNUM_P(value)) {
        return FIX2LONG(value) < 0;
    }
    return FIX2INT(rb_big_cmp(value, INT2FIX(0))) < 0;
}

/* Integers are strict: `1.5` raises instead of truncating to `1`. */
static inline long long boltffi_ruby_signed(VALUE value, long long min, long long max, const char *type) {
    if (!RB_INTEGER_TYPE_P(value)) {
        boltffi_ruby_wrong_type(value, "Integer");
    }
    long long result = NUM2LL(value);
    if (result < min || result > max) {
        rb_raise(rb_eRangeError, "integer %lld out of range for %s", result, type);
    }
    return result;
}

/* `NUM2ULL(-1)` wraps to the maximum value, so negatives are rejected first. */
static inline unsigned long long boltffi_ruby_unsigned(VALUE value, unsigned long long max, const char *type) {
    if (!RB_INTEGER_TYPE_P(value)) {
        boltffi_ruby_wrong_type(value, "Integer");
    }
    if (boltffi_ruby_negative(value)) {
        rb_raise(rb_eRangeError, "integer %" PRIsVALUE " out of range for %s", value, type);
    }
    unsigned long long result = NUM2ULL(value);
    if (result > max) {
        rb_raise(rb_eRangeError, "integer %llu out of range for %s", result, type);
    }
    return result;
}

static inline bool boltffi_ruby_to_bool(VALUE value) {
    if (value == Qtrue) {
        return true;
    }
    if (value != Qfalse) {
        boltffi_ruby_wrong_type(value, "true or false");
    }
    return false;
}

static inline int8_t boltffi_ruby_to_i8(VALUE value) { return (int8_t)boltffi_ruby_signed(value, INT8_MIN, INT8_MAX, "i8"); }
static inline int16_t boltffi_ruby_to_i16(VALUE value) { return (int16_t)boltffi_ruby_signed(value, INT16_MIN, INT16_MAX, "i16"); }
static inline int32_t boltffi_ruby_to_i32(VALUE value) { return (int32_t)boltffi_ruby_signed(value, INT32_MIN, INT32_MAX, "i32"); }
static inline int64_t boltffi_ruby_to_i64(VALUE value) { return (int64_t)boltffi_ruby_signed(value, INT64_MIN, INT64_MAX, "i64"); }
static inline intptr_t boltffi_ruby_to_isize(VALUE value) { return (intptr_t)boltffi_ruby_signed(value, INTPTR_MIN, INTPTR_MAX, "isize"); }
static inline uint8_t boltffi_ruby_to_u8(VALUE value) { return (uint8_t)boltffi_ruby_unsigned(value, UINT8_MAX, "u8"); }
static inline uint16_t boltffi_ruby_to_u16(VALUE value) { return (uint16_t)boltffi_ruby_unsigned(value, UINT16_MAX, "u16"); }
static inline uint32_t boltffi_ruby_to_u32(VALUE value) { return (uint32_t)boltffi_ruby_unsigned(value, UINT32_MAX, "u32"); }
static inline uint64_t boltffi_ruby_to_u64(VALUE value) { return (uint64_t)boltffi_ruby_unsigned(value, UINT64_MAX, "u64"); }
static inline uintptr_t boltffi_ruby_to_usize(VALUE value) { return (uintptr_t)boltffi_ruby_unsigned(value, UINTPTR_MAX, "usize"); }

/*
 * Floats accept `Float` and `Integer`. `rb_big2dbl` prints a warning in
 * verbose mode when an Integer is too large for a double, and a warning runs
 * Ruby code. So verbose mode is off during the conversion, and an Integer
 * too large for a double raises instead of becoming Infinity.
 */
static inline double boltffi_ruby_to_f64(VALUE value) {
    if (RB_FLOAT_TYPE_P(value)) {
        return RFLOAT_VALUE(value);
    }
    if (RB_FIXNUM_P(value)) {
        return (double)FIX2LONG(value);
    }
    if (!RB_INTEGER_TYPE_P(value)) {
        boltffi_ruby_wrong_type(value, "Float");
    }
    VALUE verbose = ruby_verbose;
    ruby_verbose = Qfalse;
    double result = rb_big2dbl(value);
    ruby_verbose = verbose;
    if (isinf(result)) {
        rb_raise(rb_eRangeError, "integer out of range for f64");
    }
    return result;
}

static inline float boltffi_ruby_to_f32(VALUE value) {
    double result = boltffi_ruby_to_f64(value);
    if (isfinite(result) && (result > FLT_MAX || result < -FLT_MAX)) {
        rb_raise(rb_eRangeError, "float %g out of range for f32", result);
    }
    return (float)result;
}

/* ---- C to Ruby --------------------------------------------------------- */

static inline VALUE boltffi_ruby_from_bool(bool value) { return value ? Qtrue : Qfalse; }
static inline VALUE boltffi_ruby_from_i8(int8_t value) { return INT2FIX(value); }
static inline VALUE boltffi_ruby_from_i16(int16_t value) { return INT2FIX(value); }
static inline VALUE boltffi_ruby_from_i32(int32_t value) { return INT2NUM(value); }
static inline VALUE boltffi_ruby_from_i64(int64_t value) { return LL2NUM(value); }
static inline VALUE boltffi_ruby_from_isize(intptr_t value) { return LL2NUM((long long)value); }
static inline VALUE boltffi_ruby_from_u8(uint8_t value) { return INT2FIX(value); }
static inline VALUE boltffi_ruby_from_u16(uint16_t value) { return INT2FIX(value); }
static inline VALUE boltffi_ruby_from_u32(uint32_t value) { return UINT2NUM(value); }
static inline VALUE boltffi_ruby_from_u64(uint64_t value) { return ULL2NUM(value); }
static inline VALUE boltffi_ruby_from_usize(uintptr_t value) { return ULL2NUM((unsigned long long)value); }
static inline VALUE boltffi_ruby_from_f32(float value) { return DBL2NUM((double)value); }
static inline VALUE boltffi_ruby_from_f64(double value) { return DBL2NUM(value); }

#endif
