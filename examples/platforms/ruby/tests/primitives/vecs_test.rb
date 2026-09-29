# frozen_string_literal: true

class PrimitiveVecsTest < Minitest::Test
  def test_echo_vec_u8
    assert_equal([0, 1, 2, 3].pack("C*"), Demo.echo_vec_u8([0, 1, 2, 3].pack("C*")), "case:primitives.vecs.u8.should_roundtrip_values")
  end

  def test_inc_u64_value
    assert_equal(42, Demo.inc_u64_value(41), "case:primitives.vecs.u64.should_increment_value")
  end
end
