# frozen_string_literal: true

class PrimitiveOptionsTest < Minitest::Test
  def test_optional_i32_roundtrip
    assert_equal(7, Demo.echo_optional_i32(7), "case:options.primitives.i32.should_roundtrip_some")
    assert_nil(Demo.echo_optional_i32(nil), "case:options.primitives.i32.should_roundtrip_none")
  end

  def test_unwrap_or_default_i32
    assert_equal(9, Demo.unwrap_or_default_i32(9, 4), "case:options.primitives.i32.should_unwrap_some")
    assert_equal(4, Demo.unwrap_or_default_i32(nil, 4), "case:options.primitives.i32.should_use_default_for_none")
  end

  def test_make_some_and_none_i32
    assert_equal(12, Demo.make_some_i32(12), "case:options.primitives.i32.should_make_some")
    assert_nil(Demo.make_none_i32(), "case:options.primitives.i32.should_make_none")
  end

  def test_double_if_some
    assert_equal(16, Demo.double_if_some(8), "case:options.primitives.i32.should_double_some")
    assert_nil(Demo.double_if_some(nil), "case:options.primitives.i32.should_preserve_none_when_doubling")
  end

  def test_find_even
    assert_equal(8, Demo.find_even(8), "case:options.primitives.i32.should_find_even_value")
    assert_nil(Demo.find_even(7), "case:options.primitives.i32.should_return_none_for_odd_value")
  end

  def test_optional_f64
    assert_in_delta(4.5, Demo.echo_optional_f64(4.5), 1e-12, "case:options.primitives.f64.should_roundtrip_some")
    assert_nil(Demo.echo_optional_f64(nil), "case:options.primitives.f64.should_roundtrip_none")
    assert_in_delta(3.5, Demo.find_positive_f64(3.5), 1e-12, "case:options.primitives.f64.should_find_positive_value")
    assert_nil(Demo.find_positive_f64(-0.1), "case:options.primitives.f64.should_return_none_for_non_positive_value")
  end

  def test_optional_bool
    assert_equal(true, Demo.echo_optional_bool(true), "case:options.primitives.bool.should_roundtrip_some")
    assert_nil(Demo.echo_optional_bool(nil), "case:options.primitives.bool.should_roundtrip_none")
  end

  def test_optional_i64
    assert_equal(9_007_199_254_740_993, Demo.find_positive_i64(9_007_199_254_740_993), "case:options.primitives.i64.should_find_positive_value")
    assert_nil(Demo.find_positive_i64(0), "case:options.primitives.i64.should_return_none_for_non_positive_value")
  end
end
