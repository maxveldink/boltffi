# frozen_string_literal: true

class PrimitiveVecsTest < Minitest::Test
  def test_echo_vec_i32
    assert_equal([1, 2, 3], Demo.echo_vec_i32([1, 2, 3]), "case:primitives.vecs.i32.should_roundtrip_non_empty")
    assert_equal([], Demo.echo_vec_i32([]), "case:primitives.vecs.i32.should_roundtrip_empty")
  end

  def test_sum_vec_i32
    assert_equal(60, Demo.sum_vec_i32([10, 20, 30]), "case:primitives.vecs.i32.should_sum_values")
    assert_equal(0, Demo.sum_vec_i32([]))
  end

  def test_echo_vec_f64
    values = Demo.echo_vec_f64([1.5, 2.5])
    assert_equal(2, values.length)
    assert_in_delta(1.5, values[0], 1e-12)
    assert_in_delta(2.5, values[1], 1e-12, "case:primitives.vecs.f64.should_roundtrip_values")
  end

  def test_echo_vec_bool
    assert_equal([true, false, true], Demo.echo_vec_bool([true, false, true]), "case:primitives.vecs.bool.should_roundtrip_values")
  end

  def test_echo_vec_i8
    assert_equal([-1, 0, 7], Demo.echo_vec_i8([-1, 0, 7]), "case:primitives.vecs.i8.should_roundtrip_values")
  end

  def test_echo_vec_u8
    assert_equal([0, 1, 2, 3].pack("C*"), Demo.echo_vec_u8([0, 1, 2, 3].pack("C*")), "case:primitives.vecs.u8.should_roundtrip_values")
  end

  def test_echo_vec_i16
    assert_equal([-3, 0, 9], Demo.echo_vec_i16([-3, 0, 9]), "case:primitives.vecs.i16.should_roundtrip_values")
  end

  def test_echo_vec_u16
    assert_equal([0, 10, 20], Demo.echo_vec_u16([0, 10, 20]), "case:primitives.vecs.u16.should_roundtrip_values")
  end

  def test_echo_vec_u32
    assert_equal([0, 10, 20], Demo.echo_vec_u32([0, 10, 20]), "case:primitives.vecs.u32.should_roundtrip_values")
  end

  def test_echo_vec_i64
    assert_equal([-5, 0, 8], Demo.echo_vec_i64([-5, 0, 8]), "case:primitives.vecs.i64.should_roundtrip_values")
  end

  def test_echo_vec_u64
    assert_equal([0, 1, 2], Demo.echo_vec_u64([0, 1, 2]), "case:primitives.vecs.u64.should_roundtrip_values")
  end

  def test_echo_vec_isize
    assert_equal([-2, 0, 5], Demo.echo_vec_isize([-2, 0, 5]), "case:primitives.vecs.isize.should_roundtrip_values")
  end

  def test_echo_vec_usize
    assert_equal([0, 2, 4], Demo.echo_vec_usize([0, 2, 4]), "case:primitives.vecs.usize.should_roundtrip_values")
  end

  def test_echo_vec_f32
    values = Demo.echo_vec_f32([1.25, -2.5])
    assert_equal(2, values.length)
    assert_in_delta(1.25, values[0], 1e-6)
    assert_in_delta(-2.5, values[1], 1e-6, "case:primitives.vecs.f32.should_roundtrip_values_with_tolerance")
  end

  def test_echo_vec_string
    assert_equal(["hello", "world"], Demo.echo_vec_string(["hello", "world"]), "case:primitives.vecs.string.should_roundtrip_values")
  end

  def test_vec_string_lengths
    assert_equal([2, 5], Demo.vec_string_lengths(["hi", "café"]), "case:primitives.vecs.string.should_report_utf8_byte_lengths")
  end

  def test_nested_vecs_roundtrip
    assert_equal([[1, 2], [], [-3]], Demo.echo_vec_vec_i32([[1, 2], [], [-3]]), "case:primitives.vecs.nested_i32.should_roundtrip_values")
    assert_equal([], Demo.echo_vec_vec_i32([]), "case:primitives.vecs.nested_i32.should_roundtrip_empty_outer")
    assert_equal([[true, false], [], [false]], Demo.echo_vec_vec_bool([[true, false], [], [false]]), "case:primitives.vecs.nested_bool.should_roundtrip_values")
    assert_equal([[-2, 0, 5], []], Demo.echo_vec_vec_isize([[-2, 0, 5], []]), "case:primitives.vecs.nested_isize.should_roundtrip_values")
    assert_equal([[0, 2, 4], []], Demo.echo_vec_vec_usize([[0, 2, 4], []]), "case:primitives.vecs.nested_usize.should_roundtrip_values")
    assert_equal([["hello", "café"], [], ["world"]], Demo.echo_vec_vec_string([["hello", "café"], [], ["world"]]), "case:primitives.vecs.nested_string.should_roundtrip_utf8_values")
  end

  def test_flatten_vec_vec_i32
    assert_equal([1, 2, 3], Demo.flatten_vec_vec_i32([[1, 2], [], [3]]), "case:primitives.vecs.nested_i32.should_flatten_values")
    assert_equal([], Demo.flatten_vec_vec_i32([]), "case:primitives.vecs.nested_i32.should_flatten_empty")
  end

  def test_make_range
    assert_equal([0, 1, 2, 3, 4], Demo.make_range(0, 5), "case:primitives.vecs.i32.should_make_range")
  end

  def test_reverse_vec_i32
    assert_equal([3, 2, 1], Demo.reverse_vec_i32([1, 2, 3]), "case:primitives.vecs.i32.should_reverse_values")
  end

  def test_generate_i32_vec
    assert_equal([0, 1, 2, 3], Demo.generate_i32_vec(4), "case:primitives.vecs.i32.should_generate_sequence")
  end

  def test_sum_i32_vec
    assert_equal(60, Demo.sum_i32_vec([10, 20, 30]), "case:primitives.vecs.i32.should_sum_benchmark_values")
  end

  def test_generate_f64_vec
    assert_equal([0.0, 0.1, 0.2], Demo.generate_f64_vec(3), "case:primitives.vecs.f64.should_generate_sequence")
  end

  def test_sum_f64_vec
    assert_in_delta(8.0, Demo.sum_f64_vec([1.5, 2.5, 4.0]), 1e-12, "case:primitives.vecs.f64.should_sum_values")
  end

  def test_inc_u64_value
    assert_equal(42, Demo.inc_u64_value(41), "case:primitives.vecs.u64.should_increment_value")
  end
end
