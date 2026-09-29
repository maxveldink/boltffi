# frozen_string_literal: true

# Ruby-specific rules at the native boundary. These tests have no demo case:
# they pin the conversions every Ruby caller relies on.
class ConversionsTest < Minitest::Test
  def test_integers_reject_non_integers_instead_of_truncating
    assert_raises(TypeError) { Demo.echo_i32(1.5) }
    assert_raises(TypeError) { Demo.echo_i32("1") }
    assert_raises(TypeError) { Demo.echo_i32(nil) }
  end

  def test_integers_reject_values_outside_the_rust_type
    assert_raises(RangeError) { Demo.echo_i8(128) }
    assert_raises(RangeError) { Demo.echo_i8(-129) }
    assert_raises(RangeError) { Demo.echo_i32(2**31) }
    assert_raises(RangeError) { Demo.echo_i64(2**63) }
    assert_equal(-2**63, Demo.echo_i64(-2**63))
  end

  def test_unsigned_integers_reject_negative_values
    assert_raises(RangeError) { Demo.echo_u8(-1) }
    assert_raises(RangeError) { Demo.echo_u32(-1) }
    assert_raises(RangeError) { Demo.echo_u64(-1) }
    assert_raises(RangeError) { Demo.echo_usize(-1) }
    assert_raises(RangeError) { Demo.echo_u64(2**64) }
    assert_equal(2**64 - 1, Demo.echo_u64(2**64 - 1))
  end

  def test_floats_accept_integers_and_reject_other_types
    assert_equal(3.0, Demo.echo_f64(3))
    assert_raises(TypeError) { Demo.echo_f64("3.0") }
    assert_raises(TypeError) { Demo.echo_f64(nil) }
    assert_raises(RangeError) { Demo.echo_f32(1e39) }
    assert_equal(Float::INFINITY, Demo.echo_f32(Float::INFINITY))
  end

  # Ruby warns when an Integer is too large for a Float. A warning hook is Ruby
  # code, and no Ruby code may run while an argument is encoded.
  def test_booleans_are_strict
    assert_raises(TypeError) { Demo.echo_bool(nil) }
    assert_raises(TypeError) { Demo.echo_bool(1) }
  end

  # Encoding runs no Ruby code: a `to_str` method could change a Hash or an
  # Array after the extension wrote its size.
end
