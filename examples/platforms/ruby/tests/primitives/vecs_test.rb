# frozen_string_literal: true

class PrimitiveVecsTest < Minitest::Test
  def test_inc_u64_value
    assert_equal(42, Demo.inc_u64_value(41), "case:primitives.vecs.u64.should_increment_value")
  end
end
