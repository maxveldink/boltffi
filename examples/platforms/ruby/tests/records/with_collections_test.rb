# frozen_string_literal: true

class CollectionRecordTest < Minitest::Test
  def assert_point(point, x:, y:, tolerance: 1e-12)
    assert_in_delta(x, point.x, tolerance)
    assert_in_delta(y, point.y, tolerance)
  end

  def test_polygon
    points = [Demo::Point.new(x: 0.0, y: 0.0), Demo::Point.new(x: 1.0, y: 0.0), Demo::Point.new(x: 0.0, y: 1.0)]
    polygon = Demo.make_polygon(points)
    assert_equal(Demo::Polygon.new(points: points), polygon, "case:records.with_collections.polygon.should_make_from_points")
    assert_equal(polygon, Demo.echo_polygon(polygon), "case:records.with_collections.polygon.should_roundtrip_point_vector")
    assert_equal(3, Demo.polygon_vertex_count(polygon), "case:records.with_collections.polygon.should_report_vertex_count")
    centroid = Demo.polygon_centroid(polygon)
    assert_in_delta(1.0 / 3.0, centroid.x, 1e-6, "case:records.with_collections.polygon.should_compute_centroid")
    assert_in_delta(1.0 / 3.0, centroid.y, 1e-6)
  end

  def test_team
    team = Demo.make_team("devs", ["Ali", "Mia"])
    assert_equal(Demo::Team.new(name: "devs", members_: ["Ali", "Mia"]), team, "case:records.with_collections.team.should_make_from_members")
    assert_equal(team, Demo.echo_team(team), "case:records.with_collections.team.should_roundtrip_member_vector")
    assert_equal(2, Demo.team_size(team), "case:records.with_collections.team.should_report_member_count")
  end

  def test_classroom
    students = [Demo::Person.new(name: "Mia", age: 10), Demo::Person.new(name: "Leo", age: 11)]
    classroom = Demo.make_classroom(students)
    assert_equal(Demo::Classroom.new(students: students), classroom, "case:records.with_collections.classroom.should_make_from_students")
    assert_equal(classroom, Demo.echo_classroom(classroom), "case:records.with_collections.classroom.should_roundtrip_student_vector")
  end

  def test_tagged_scores
    tagged_scores = Demo::TaggedScores.new(label: "math", scores: [90.0, 85.5])
    assert_equal(tagged_scores, Demo.echo_tagged_scores(tagged_scores), "case:records.with_collections.tagged_scores.should_roundtrip_score_vector")
    assert_in_delta(90.0, Demo.average_score(Demo::TaggedScores.new(label: "x", scores: [80.0, 100.0])), 1e-12, "case:records.with_collections.tagged_scores.should_average_scores")
  end

  def test_user_profiles
    profiles = Demo.generate_user_profiles(4)
    assert_equal(4, profiles.length, "case:records.with_collections.user_profiles.should_generate_profiles")
    assert_equal(0, profiles[0].id)
    assert_equal(3, profiles[3].id)
    expected_score_sum = profiles.sum(&:score)
    assert_in_delta(expected_score_sum, Demo.sum_user_scores(profiles), 1e-4, "case:records.with_collections.user_profiles.should_sum_scores")
    assert_equal(profiles.count(&:is_active), Demo.count_active_users(profiles), "case:records.with_collections.user_profiles.should_count_active_users")
  end
end
