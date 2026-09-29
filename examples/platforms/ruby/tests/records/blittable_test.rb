# frozen_string_literal: true

class BlittableRecordsTest < Minitest::Test
  def assert_point(point, x:, y:, tolerance: 1e-12)
    assert_kind_of(Demo::Point, point)
    assert_in_delta(x, point.x, tolerance)
    assert_in_delta(y, point.y, tolerance)
  end

  def test_point_functions
    point = Demo::Point.new(x: 1.0, y: 2.0)
    assert_point(Demo.echo_point(point), x: 1.0, y: 2.0)
    assert_equal(point, Demo.echo_point(point), "case:records.blittable.point.should_roundtrip_value")
    assert_point(Demo.make_point(1.0, 2.0), x: 1.0, y: 2.0)
    assert_equal(Demo::Point.new(x: 1.0, y: 2.0), Demo.make_point(1.0, 2.0), "case:records.blittable.point.should_make_from_coordinates")
    assert_point(
      Demo.add_points(Demo::Point.new(x: 3.0, y: 4.0), Demo::Point.new(x: 5.0, y: 6.0)),
      x: 8.0, y: 10.0,
    )
    assert_equal(
      Demo::Point.new(x: 8.0, y: 10.0),
      Demo.add_points(Demo::Point.new(x: 3.0, y: 4.0), Demo::Point.new(x: 5.0, y: 6.0)),
      "case:records.blittable.point.should_add_values",
    )
    assert_point(Demo.try_make_point(2.0, 3.0), x: 2.0, y: 3.0)
    assert_equal(Demo::Point.new(x: 2.0, y: 3.0), Demo.try_make_point(2.0, 3.0), "case:records.blittable.point.should_return_some_for_nonzero_coordinates")
    assert_nil(Demo.try_make_point(0.0, 0.0), "case:records.blittable.point.should_return_none_for_origin_coordinates")
  end

  def test_color_functions
    color = Demo::Color.new(r: 1, g: 2, b: 3, a: 255)
    assert_equal(color, Demo.echo_color(color), "case:records.blittable.color.should_roundtrip_value")
    assert_equal(Demo::Color.new(r: 9, g: 8, b: 7, a: 6), Demo.make_color(9, 8, 7, 6), "case:records.blittable.color.should_make_from_channels")
  end

  def test_location_vectors
    locations = Demo.generate_locations(3)
    assert_equal(3, locations.length, "case:records.blittable.locations.should_generate_sample_vector")
    assert_equal(3, Demo.process_locations(locations), "case:records.blittable.locations.should_count_vector_items")
    assert_equal(0, Demo.process_locations([]), "case:records.blittable.locations.should_count_empty_vector")

    host_locations = [
      Demo::Location.new(id: 1, lat: 1.0, lng: 2.0, rating: 3.5, review_count: 4, is_open: true),
      Demo::Location.new(id: 2, lat: 5.0, lng: 6.0, rating: 2.5, review_count: 8, is_open: false),
    ]
    assert_equal(2, Demo.process_locations(host_locations), "case:records.blittable.locations.should_count_host_constructed_vector")
    assert_in_delta(9.3, Demo.sum_ratings(locations), 1e-4, "case:records.blittable.locations.should_sum_generated_ratings")
    assert_in_delta(6.0, Demo.sum_ratings(host_locations), 1e-4, "case:records.blittable.locations.should_sum_host_constructed_ratings")

    found_location = Demo.find_location(7)
    refute_nil(found_location)
    assert_equal(7, found_location.id, "case:records.blittable.locations.find_location.should_return_some_for_positive_id")
    assert_nil(Demo.find_location(0), "case:records.blittable.locations.find_location.should_return_none_for_non_positive_id")
    found_locations = Demo.find_locations(3)
    refute_nil(found_locations)
    assert_equal(3, found_locations.length, "case:records.blittable.locations.find_locations.should_return_some_vector_for_positive_count")
    assert_nil(Demo.find_locations(0), "case:records.blittable.locations.find_locations.should_return_none_for_non_positive_count")
  end

  def test_trade_particle_and_sensor_vectors
    trades = Demo.generate_trades(3)
    assert_equal(3, trades.length, "case:records.blittable.trades.should_generate_sample_vector")
    assert_equal(3000, Demo.sum_trade_volumes(trades), "case:records.blittable.trades.should_sum_volumes")
    assert_equal(3002, Demo.aggregate_location_trade_stats(Demo.generate_locations(3), trades), "case:records.blittable.trades.should_aggregate_with_locations")

    particles = Demo.generate_particles(3)
    assert_equal(3, particles.length, "case:records.blittable.particles.should_generate_sample_vector")
    assert_in_delta(3.003, Demo.sum_particle_masses(particles), 1e-4, "case:records.blittable.particles.should_sum_masses")

    readings = Demo.generate_sensor_readings(3)
    assert_equal(3, readings.length, "case:records.blittable.sensor_readings.should_generate_sample_vector")
    assert_in_delta(21.0, Demo.avg_sensor_temperature(readings), 1e-4, "case:records.blittable.sensor_readings.should_average_generated_temperatures")
    assert_in_delta(0.0, Demo.avg_sensor_temperature([]), 1e-12, "case:records.blittable.sensor_readings.should_average_empty_vector_as_zero")
  end
end
