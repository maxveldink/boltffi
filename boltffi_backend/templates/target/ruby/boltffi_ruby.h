/*
 * BoltFFI Ruby runtime. Generated extensions include this file.
 *
 * Every value crosses the boundary as a plain Ruby object. The helpers here
 * follow five rules:
 *
 * 1. Convert and check every Ruby argument before Rust runs. A Ruby exception
 *    is a longjmp, so C code must not own memory that a raise would skip.
 * 2. Encode arguments into a writer that starts on the C stack and moves into
 *    a hidden, GC-owned Ruby String when it grows. A raise leaks nothing.
 * 3. Free every buffer that Rust returns exactly once, also when decoding
 *    raises. `boltffi_ruby_decode_owned` runs the decoder under `rb_ensure`.
 * 4. Send Rust only valid UTF-8. Rust builds `String` values from these bytes
 *    without a second check.
 * 5. Run no Ruby code while an argument is encoded. Each check reads the type
 *    and never calls a conversion method such as `to_str`. Ruby code could
 *    change a Hash or an Array after its size is written.
 */
#ifndef BOLTFFI_RUBY_H
#define BOLTFFI_RUBY_H

#include <float.h>
#include <limits.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

#include <ruby.h>
#include <ruby/encoding.h>

#include "boltffi.h"

#if defined(__BYTE_ORDER__) && defined(__ORDER_BIG_ENDIAN__) && __BYTE_ORDER__ == __ORDER_BIG_ENDIAN__
#error "BoltFFI Ruby extensions require a little-endian target"
#endif

/* Record helpers exist for every record, even when no function uses one. */
#if defined(__GNUC__) || defined(__clang__)
#define BOLTFFI_RUBY_MAYBE_UNUSED __attribute__((unused))
#else
#define BOLTFFI_RUBY_MAYBE_UNUSED
#endif

/* ---- errors ------------------------------------------------------------ */

NORETURN(static void boltffi_ruby_malformed(void));
static void boltffi_ruby_malformed(void) {
    rb_raise(rb_eRuntimeError, "BoltFFI: the native library returned a malformed value");
}

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

/* Floats accept `Float` and `Integer`. */
static inline double boltffi_ruby_to_f64(VALUE value) {
    if (RB_FLOAT_TYPE_P(value)) {
        return RFLOAT_VALUE(value);
    }
    if (!RB_INTEGER_TYPE_P(value)) {
        boltffi_ruby_wrong_type(value, "Float");
    }
    return NUM2DBL(value);
}

static inline float boltffi_ruby_to_f32(VALUE value) {
    double result = boltffi_ruby_to_f64(value);
    if (isfinite(result) && (result > FLT_MAX || result < -FLT_MAX)) {
        rb_raise(rb_eRangeError, "float %g out of range for f32", result);
    }
    return (float)result;
}

/* Strings are strict: an object with `to_str` raises instead of converting. */
static inline VALUE boltffi_ruby_expect_string(VALUE value) {
    if (!RB_TYPE_P(value, T_STRING)) {
        boltffi_ruby_wrong_type(value, "String");
    }
    return value;
}

/*
 * Returns a String whose bytes are valid UTF-8.
 *
 * UTF-8 strings pass after a validity check. ASCII-only strings in another
 * ASCII-compatible encoding pass because their bytes are already UTF-8. Every
 * other string raises: transcoding is the caller's choice, not ours.
 */
static inline VALUE boltffi_ruby_utf8(VALUE value) {
    boltffi_ruby_expect_string(value);
    int encoding = ENCODING_GET(value);
    int coderange = rb_enc_str_coderange(value);
    if (coderange == ENC_CODERANGE_BROKEN) {
        rb_raise(rb_eArgError, "invalid byte sequence in %s", rb_enc_name(rb_enc_from_index(encoding)));
    }
    if (encoding != rb_utf8_encindex() && (coderange != ENC_CODERANGE_7BIT || !rb_enc_asciicompat(rb_enc_from_index(encoding)))) {
        rb_raise(rb_eEncCompatError, "BoltFFI strings must be UTF-8, got %s", rb_enc_name(rb_enc_from_index(encoding)));
    }
    return value;
}

static inline VALUE boltffi_ruby_expect_array(VALUE value) {
    if (!RB_TYPE_P(value, T_ARRAY)) {
        boltffi_ruby_wrong_type(value, "Array");
    }
    return value;
}

static inline VALUE boltffi_ruby_expect_hash(VALUE value) {
    if (!RB_TYPE_P(value, T_HASH)) {
        boltffi_ruby_wrong_type(value, "Hash");
    }
    return value;
}

static inline void boltffi_ruby_expect_tuple(VALUE value, long size) {
    boltffi_ruby_expect_array(value);
    if (RARRAY_LEN(value) != size) {
        rb_raise(rb_eArgError, "expected an Array of %ld elements, got %ld", size, RARRAY_LEN(value));
    }
}

static inline void boltffi_ruby_expect_record(VALUE value, VALUE record_class) {
    if (!RTEST(rb_obj_is_kind_of(value, record_class))) {
        rb_raise(rb_eTypeError, "wrong argument type %" PRIsVALUE " (expected %" PRIsVALUE ")", rb_obj_class(value), record_class);
    }
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

/* ---- reading values that Rust encoded --------------------------------- */

typedef struct {
    const uint8_t *ptr;
    uintptr_t len;
    uintptr_t offset;
} boltffi_ruby_reader;

static inline const uint8_t *boltffi_ruby_read_bytes(boltffi_ruby_reader *reader, uintptr_t count) {
    if (count > reader->len - reader->offset) {
        boltffi_ruby_malformed();
    }
    const uint8_t *bytes = reader->ptr + reader->offset;
    reader->offset += count;
    return bytes;
}

#define BOLTFFI_RUBY_READ_SCALAR(name, type)                                        \
    static inline type boltffi_ruby_read_raw_##name(boltffi_ruby_reader *reader) {  \
        type value;                                                                 \
        memcpy(&value, boltffi_ruby_read_bytes(reader, sizeof(value)), sizeof(value)); \
        return value;                                                               \
    }

BOLTFFI_RUBY_READ_SCALAR(u8, uint8_t)
BOLTFFI_RUBY_READ_SCALAR(i8, int8_t)
BOLTFFI_RUBY_READ_SCALAR(u16, uint16_t)
BOLTFFI_RUBY_READ_SCALAR(i16, int16_t)
BOLTFFI_RUBY_READ_SCALAR(u32, uint32_t)
BOLTFFI_RUBY_READ_SCALAR(i32, int32_t)
BOLTFFI_RUBY_READ_SCALAR(u64, uint64_t)
BOLTFFI_RUBY_READ_SCALAR(i64, int64_t)
BOLTFFI_RUBY_READ_SCALAR(f32, float)
BOLTFFI_RUBY_READ_SCALAR(f64, double)

#undef BOLTFFI_RUBY_READ_SCALAR

static inline VALUE boltffi_ruby_read_bool(boltffi_ruby_reader *reader) {
    uint8_t value = boltffi_ruby_read_raw_u8(reader);
    if (value > 1) {
        boltffi_ruby_malformed();
    }
    return value ? Qtrue : Qfalse;
}

static inline VALUE boltffi_ruby_read_i8(boltffi_ruby_reader *reader) { return boltffi_ruby_from_i8(boltffi_ruby_read_raw_i8(reader)); }
static inline VALUE boltffi_ruby_read_i16(boltffi_ruby_reader *reader) { return boltffi_ruby_from_i16(boltffi_ruby_read_raw_i16(reader)); }
static inline VALUE boltffi_ruby_read_i32(boltffi_ruby_reader *reader) { return boltffi_ruby_from_i32(boltffi_ruby_read_raw_i32(reader)); }
static inline VALUE boltffi_ruby_read_i64(boltffi_ruby_reader *reader) { return boltffi_ruby_from_i64(boltffi_ruby_read_raw_i64(reader)); }
/* `isize` and `usize` always cross as 8 bytes. */
static inline VALUE boltffi_ruby_read_isize(boltffi_ruby_reader *reader) { return LL2NUM((long long)boltffi_ruby_read_raw_i64(reader)); }
static inline VALUE boltffi_ruby_read_u8(boltffi_ruby_reader *reader) { return boltffi_ruby_from_u8(boltffi_ruby_read_raw_u8(reader)); }
static inline VALUE boltffi_ruby_read_u16(boltffi_ruby_reader *reader) { return boltffi_ruby_from_u16(boltffi_ruby_read_raw_u16(reader)); }
static inline VALUE boltffi_ruby_read_u32(boltffi_ruby_reader *reader) { return boltffi_ruby_from_u32(boltffi_ruby_read_raw_u32(reader)); }
static inline VALUE boltffi_ruby_read_u64(boltffi_ruby_reader *reader) { return boltffi_ruby_from_u64(boltffi_ruby_read_raw_u64(reader)); }
static inline VALUE boltffi_ruby_read_usize(boltffi_ruby_reader *reader) { return ULL2NUM((unsigned long long)boltffi_ruby_read_raw_u64(reader)); }
static inline VALUE boltffi_ruby_read_f32(boltffi_ruby_reader *reader) { return boltffi_ruby_from_f32(boltffi_ruby_read_raw_f32(reader)); }
static inline VALUE boltffi_ruby_read_f64(boltffi_ruby_reader *reader) { return boltffi_ruby_from_f64(boltffi_ruby_read_raw_f64(reader)); }

/*
 * A direct vector holds `isize` and `usize` at native width. The wire format
 * always spends 8 bytes on them, so the wire readers above do not apply.
 */
static inline VALUE boltffi_ruby_read_native_isize(boltffi_ruby_reader *reader) {
    intptr_t value;
    memcpy(&value, boltffi_ruby_read_bytes(reader, sizeof(value)), sizeof(value));
    return boltffi_ruby_from_isize(value);
}

static inline VALUE boltffi_ruby_read_native_usize(boltffi_ruby_reader *reader) {
    uintptr_t value;
    memcpy(&value, boltffi_ruby_read_bytes(reader, sizeof(value)), sizeof(value));
    return boltffi_ruby_from_usize(value);
}

static inline bool boltffi_ruby_read_option_tag(boltffi_ruby_reader *reader) {
    uint8_t tag = boltffi_ruby_read_raw_u8(reader);
    if (tag > 1) {
        boltffi_ruby_malformed();
    }
    return tag == 1;
}

/*
 * Reads an element count. Each element takes at least `minimum_size` bytes,
 * so a count that cannot fit in the rest of the buffer is rejected before
 * Ruby allocates an Array for it.
 */
static inline long boltffi_ruby_read_count(boltffi_ruby_reader *reader, uintptr_t minimum_size) {
    uint32_t count = boltffi_ruby_read_raw_u32(reader);
    if (minimum_size != 0 && count > (reader->len - reader->offset) / minimum_size) {
        boltffi_ruby_malformed();
    }
    return (long)count;
}

static inline VALUE boltffi_ruby_read_string(boltffi_ruby_reader *reader) {
    uint32_t len = boltffi_ruby_read_raw_u32(reader);
    const uint8_t *bytes = boltffi_ruby_read_bytes(reader, len);
    return rb_utf8_str_new((const char *)bytes, (long)len);
}

static inline VALUE boltffi_ruby_read_binary(boltffi_ruby_reader *reader) {
    uint32_t len = boltffi_ruby_read_raw_u32(reader);
    const uint8_t *bytes = boltffi_ruby_read_bytes(reader, len);
    return rb_str_new((const char *)bytes, (long)len);
}

/* A direct vector returns raw elements, so its length must be whole elements. */
static inline long boltffi_ruby_raw_count(boltffi_ruby_reader *reader, uintptr_t element_size) {
    if (reader->len % element_size != 0 || reader->len / element_size > (uintptr_t)LONG_MAX) {
        boltffi_ruby_malformed();
    }
    return (long)(reader->len / element_size);
}

typedef VALUE boltffi_ruby_decode_fn(boltffi_ruby_reader *reader);

typedef struct {
    FfiBuf_u8 buffer;
    boltffi_ruby_decode_fn *decode;
} boltffi_ruby_owned;

static VALUE boltffi_ruby_owned_decode(VALUE argument) {
    boltffi_ruby_owned *owned = (boltffi_ruby_owned *)argument;
    if (owned->buffer.ptr == NULL && owned->buffer.len != 0) {
        boltffi_ruby_malformed();
    }
    boltffi_ruby_reader reader = { owned->buffer.ptr, owned->buffer.len, 0 };
    VALUE value = owned->decode(&reader);
    if (reader.offset != reader.len) {
        boltffi_ruby_malformed();
    }
    return value;
}

static VALUE boltffi_ruby_owned_free(VALUE argument) {
    boltffi_free_buf(((boltffi_ruby_owned *)argument)->buffer);
    return Qnil;
}

/* Decodes a buffer that Rust returned and frees it, also when decoding raises. */
static inline VALUE boltffi_ruby_decode_owned(FfiBuf_u8 buffer, boltffi_ruby_decode_fn *decode) {
    boltffi_ruby_owned owned = { buffer, decode };
    return rb_ensure(boltffi_ruby_owned_decode, (VALUE)&owned, boltffi_ruby_owned_free, (VALUE)&owned);
}

/*
 * Rust reports an argument it cannot decode through the last-error slot and
 * returns a zero value. Each call that sends encoded bytes checks the slot, so
 * the failure raises instead of returning that zero value. `result` is the
 * buffer the call returned, or NULL; it is freed before the raise.
 */
static void boltffi_ruby_check_arguments(FfiBuf_u8 *result) {
    FfiString message = { NULL, 0, 0 };
    boltffi_last_error_message(&message);
    if (message.len == 0) {
        boltffi_free_string(message);
        return;
    }
    if (result != NULL) {
        boltffi_free_buf(*result);
    }
    char text[256];
    size_t len = message.len < sizeof(text) - 1 ? (size_t)message.len : sizeof(text) - 1;
    memcpy(text, message.ptr, len);
    text[len] = '\0';
    boltffi_free_string(message);
    rb_raise(rb_eArgError, "BoltFFI: the native library rejected an argument: %s", text);
}

/* ---- writing values for Rust ------------------------------------------- */

#define BOLTFFI_RUBY_WRITER_INLINE_CAPACITY 128

/*
 * Bytes for one encoded argument.
 *
 * Small arguments stay in `inline_bytes` on the C stack. A larger argument
 * moves into `heap`, a hidden Ruby String the GC owns, so a raise in the
 * middle of encoding leaks nothing. `ObjectSpace` cannot find a hidden
 * object, so no Ruby code can change the bytes. The writer lives on the C stack, so the GC finds
 * and pins `heap` while the writer is in scope. Call `RB_GC_GUARD` on `heap`
 * after the native call that reads the bytes.
 */
typedef struct {
    uint8_t *ptr;
    uintptr_t len;
    uintptr_t capacity;
    VALUE heap;
    uint8_t inline_bytes[BOLTFFI_RUBY_WRITER_INLINE_CAPACITY];
} boltffi_ruby_writer;

static inline void boltffi_ruby_writer_init(boltffi_ruby_writer *writer) {
    writer->ptr = writer->inline_bytes;
    writer->len = 0;
    writer->capacity = sizeof(writer->inline_bytes);
    writer->heap = Qfalse;
}

static void boltffi_ruby_writer_grow(boltffi_ruby_writer *writer, uintptr_t additional) {
    if (additional > (uintptr_t)LONG_MAX - writer->len) {
        rb_raise(rb_eArgError, "BoltFFI: argument is too large to encode");
    }
    uintptr_t needed = writer->len + additional;
    uintptr_t capacity = writer->capacity;
    while (capacity < needed) {
        capacity = capacity > (uintptr_t)LONG_MAX / 2 ? (uintptr_t)LONG_MAX : capacity * 2;
    }
    if (writer->heap == Qfalse) {
        VALUE heap = rb_str_tmp_new((long)capacity);
        memcpy(RSTRING_PTR(heap), writer->ptr, writer->len);
        writer->heap = heap;
    } else {
        rb_str_resize(writer->heap, (long)capacity);
    }
    writer->ptr = (uint8_t *)RSTRING_PTR(writer->heap);
    writer->capacity = capacity;
}

static inline uint8_t *boltffi_ruby_writer_reserve(boltffi_ruby_writer *writer, uintptr_t count) {
    if (count > writer->capacity - writer->len) {
        boltffi_ruby_writer_grow(writer, count);
    }
    uint8_t *slot = writer->ptr + writer->len;
    writer->len += count;
    return slot;
}

static inline void boltffi_ruby_write_raw(boltffi_ruby_writer *writer, const void *bytes, uintptr_t count) {
    uint8_t *slot = boltffi_ruby_writer_reserve(writer, count);
    if (count != 0) {
        memcpy(slot, bytes, count);
    }
}

static inline void boltffi_ruby_write_raw_u8(boltffi_ruby_writer *writer, uint8_t value) {
    *boltffi_ruby_writer_reserve(writer, 1) = value;
}

#define BOLTFFI_RUBY_WRITE_SCALAR(name, type, wire_type)                        \
    static inline void boltffi_ruby_write_##name(boltffi_ruby_writer *writer, VALUE value) { \
        wire_type wire = (wire_type)boltffi_ruby_to_##name(value);              \
        boltffi_ruby_write_raw(writer, &wire, sizeof(wire));                    \
    }

BOLTFFI_RUBY_WRITE_SCALAR(i8, int8_t, int8_t)
BOLTFFI_RUBY_WRITE_SCALAR(i16, int16_t, int16_t)
BOLTFFI_RUBY_WRITE_SCALAR(i32, int32_t, int32_t)
BOLTFFI_RUBY_WRITE_SCALAR(i64, int64_t, int64_t)
BOLTFFI_RUBY_WRITE_SCALAR(isize, intptr_t, int64_t)
BOLTFFI_RUBY_WRITE_SCALAR(u8, uint8_t, uint8_t)
BOLTFFI_RUBY_WRITE_SCALAR(u16, uint16_t, uint16_t)
BOLTFFI_RUBY_WRITE_SCALAR(u32, uint32_t, uint32_t)
BOLTFFI_RUBY_WRITE_SCALAR(u64, uint64_t, uint64_t)
BOLTFFI_RUBY_WRITE_SCALAR(usize, uintptr_t, uint64_t)
BOLTFFI_RUBY_WRITE_SCALAR(f32, float, float)
BOLTFFI_RUBY_WRITE_SCALAR(f64, double, double)

#undef BOLTFFI_RUBY_WRITE_SCALAR

static inline void boltffi_ruby_write_bool(boltffi_ruby_writer *writer, VALUE value) {
    boltffi_ruby_write_raw_u8(writer, boltffi_ruby_to_bool(value) ? 1 : 0);
}

static inline void boltffi_ruby_write_count(boltffi_ruby_writer *writer, long count) {
    if (count < 0 || (unsigned long)count > UINT32_MAX) {
        rb_raise(rb_eArgError, "BoltFFI: collection has too many elements");
    }
    uint32_t wire = (uint32_t)count;
    boltffi_ruby_write_raw(writer, &wire, sizeof(wire));
}

static inline void boltffi_ruby_write_prefixed(boltffi_ruby_writer *writer, VALUE string) {
    long len = RSTRING_LEN(string);
    boltffi_ruby_write_count(writer, len);
    uint8_t *slot = boltffi_ruby_writer_reserve(writer, (uintptr_t)len);
    if (len != 0) {
        memcpy(slot, RSTRING_PTR(string), (size_t)len);
    }
    RB_GC_GUARD(string);
}

static inline void boltffi_ruby_write_string(boltffi_ruby_writer *writer, VALUE value) {
    boltffi_ruby_write_prefixed(writer, boltffi_ruby_utf8(value));
}

static inline void boltffi_ruby_write_binary(boltffi_ruby_writer *writer, VALUE value) {
    boltffi_ruby_write_prefixed(writer, boltffi_ruby_expect_string(value));
}

#endif
