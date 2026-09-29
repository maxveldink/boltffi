# frozen_string_literal: true

class BlittableRecordsTest < Minitest::Test
  def assert_point(point, x:, y:, tolerance: 1e-12)
    assert_kind_of(Demo::Point, point)
    assert_in_delta(x, point.x, tolerance)
    assert_in_delta(y, point.y, tolerance)
  end

  def test_color_functions
    color = Demo::Color.new(r: 1, g: 2, b: 3, a: 255)
    assert_equal(color, Demo.echo_color(color), "case:records.blittable.color.should_roundtrip_value")
    assert_equal(Demo::Color.new(r: 9, g: 8, b: 7, a: 6), Demo.make_color(9, 8, 7, 6), "case:records.blittable.color.should_make_from_channels")
  end
end
