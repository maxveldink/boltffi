# frozen_string_literal: true

class NestedRecordTest < Minitest::Test
  def test_line
    line = Demo.make_line(0.0, 0.0, 3.0, 4.0)
    assert_equal(
      Demo::Line.new(start: Demo::Point.new(x: 0.0, y: 0.0), end: Demo::Point.new(x: 3.0, y: 4.0)),
      line,
      "case:records.nested.line.should_make_from_coordinates",
    )
    assert_equal(line, Demo.echo_line(line), "case:records.nested.line.should_roundtrip_nested_points")
    assert_in_delta(5.0, Demo.line_length(line), 1e-12, "case:records.nested.line.should_compute_length")
  end

  def test_rect
    rect = Demo::Rect.new(origin: Demo::Point.new(x: 1.0, y: 2.0), dimensions: Demo::Dimensions.new(width: 3.0, height: 4.0))
    assert_equal(rect, Demo.echo_rect(rect), "case:records.nested.rect.should_roundtrip_nested_records")
    assert_in_delta(12.0, Demo.rect_area(rect), 1e-12, "case:records.nested.rect.should_compute_area")
  end
end
