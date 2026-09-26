# frozen_string_literal: true

class BytesTest < Minitest::Test
  def test_echo_bytes
    assert_equal([1, 2, 3, 4].pack("C*"), Demo.echo_bytes([1, 2, 3, 4].pack("C*")), "case:bytes.bytes.should_roundtrip_values")
  end

  def test_bytes_length
    assert_equal(3, Demo.bytes_length([10, 20, 30].pack("C*")), "case:bytes.bytes.should_report_length")
  end

  def test_bytes_sum
    assert_equal(10, Demo.bytes_sum([1, 2, 3, 4].pack("C*")), "case:bytes.bytes.should_sum_values")
  end

  def test_make_bytes
    assert_equal([0, 1, 2, 3, 4].pack("C*"), Demo.make_bytes(5), "case:bytes.bytes.should_make_sequential_values")
  end

  def test_reverse_bytes
    assert_equal([7, 6, 5].pack("C*"), Demo.reverse_bytes([5, 6, 7].pack("C*")), "case:bytes.bytes.should_reverse_values")
  end
end
