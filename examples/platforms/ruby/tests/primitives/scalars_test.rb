# frozen_string_literal: true

class ScalarsTest < Minitest::Test
  def test_echo_bool
    assert_equal(true, Demo.echo_bool(true), "case:primitives.scalars.bool.should_roundtrip_true")
  end

  def test_negate_bool
    assert_equal(true, Demo.negate_bool(false), "case:primitives.scalars.bool.should_negate_false_to_true")
  end

  def test_echo_i8
    assert_equal(-7, Demo.echo_i8(-7), "case:primitives.scalars.i8.should_roundtrip_negative_value")
  end

  def test_echo_u8
    assert_equal(255, Demo.echo_u8(255), "case:primitives.scalars.u8.should_roundtrip_max_value")
  end

  def test_echo_i16
    assert_equal(-1234, Demo.echo_i16(-1234), "case:primitives.scalars.i16.should_roundtrip_negative_value")
  end

  def test_echo_u16
    assert_equal(55_000, Demo.echo_u16(55_000), "case:primitives.scalars.u16.should_roundtrip_large_value")
  end

  def test_echo_i32
    assert_equal(-42, Demo.echo_i32(-42), "case:primitives.scalars.i32.should_roundtrip_negative_value")
  end

  def test_add_i32
    assert_equal(30, Demo.add_i32(10, 20), "case:primitives.scalars.i32.should_add_two_values")
  end

  def test_add_alias
    assert_equal(30, Demo.add(10, 20), "case:primitives.scalars.i32.should_add_with_benchmark_alias")
  end

  def test_parameters_named_status_do_not_collide
    assert_nil(Demo.notify_status_collision(1, 2), "case:primitives.scalars.named_status.should_accept_both_names")
  end

  def test_echo_u32
    assert_equal(4_000_000_000, Demo.echo_u32(4_000_000_000), "case:primitives.scalars.u32.should_roundtrip_large_value")
  end

  def test_echo_i64
    assert_equal(-9_999_999_999, Demo.echo_i64(-9_999_999_999), "case:primitives.scalars.i64.should_roundtrip_large_negative_value")
  end

  def test_echo_u64
    assert_equal(9_999_999_999, Demo.echo_u64(9_999_999_999), "case:primitives.scalars.u64.should_roundtrip_large_value")
  end

  def test_echo_f32
    assert_in_delta(3.5, Demo.echo_f32(3.5), 1e-6, "case:primitives.scalars.f32.should_roundtrip_value_with_tolerance")
  end

  def test_add_f32
    assert_in_delta(4.0, Demo.add_f32(1.5, 2.5), 1e-6, "case:primitives.scalars.f32.should_add_two_values_with_tolerance")
  end

  def test_echo_f64
    assert_in_delta(3.14159265359, Demo.echo_f64(3.14159265359), 1e-12, "case:primitives.scalars.f64.should_roundtrip_pi_with_tolerance")
  end

  def test_add_f64
    assert_in_delta(4.0, Demo.add_f64(1.5, 2.5), 1e-12, "case:primitives.scalars.f64.should_add_two_values_with_tolerance")
  end

  def test_multiply_f64
    assert_in_delta(3.75, Demo.multiply(1.5, 2.5), 1e-12, "case:primitives.scalars.f64.should_multiply_two_values")
  end

  def test_echo_usize
    assert_equal(123, Demo.echo_usize(123), "case:primitives.scalars.usize.should_roundtrip_value")
  end

  def test_echo_isize
    assert_equal(-123, Demo.echo_isize(-123), "case:primitives.scalars.isize.should_roundtrip_negative_value")
  end

  def test_noop
    assert_nil(Demo.noop(), "case:primitives.scalars.noop.should_cross_without_values")
  end
end
