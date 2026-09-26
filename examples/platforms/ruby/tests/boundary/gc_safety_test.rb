# frozen_string_literal: true

# The extension holds Ruby objects in C locals while it builds values. These
# tests make the garbage collector run and move objects during those calls,
# so a value that the extension fails to keep alive corrupts a result or
# crashes the process.
class GcSafetyTest < Minitest::Test
  def test_values_survive_garbage_collection_on_every_allocation
    with_gc_stress do
      assert_equal(["a", "b" * 300], Demo.echo_vec_string(["a", "b" * 300]))
      assert_equal({ "k" => [1, 2, 3] }, Demo.echo_hash_map({ "k" => [1, 2, 3] }))
      assert_equal(Demo::Person.new(name: "Ada", age: 36), Demo.echo_person(Demo::Person.new(name: "Ada", age: 36)))
      assert_equal([Demo::Point.new(x: 1.0, y: 2.0), nil], [Demo.make_some_point(1.0, 2.0), Demo.make_none_point])
    end
  end

  def test_record_classes_survive_compaction
    GC.verify_compaction_references(expand_heap: true, toward: :empty)

    assert_equal(Demo::Point.new(x: 1.0, y: 2.0), Demo.make_point(1.0, 2.0))
    assert_equal(Demo::Person.new(name: "Ada", age: 36), Demo.make_person("Ada", 36))
  end

  def test_a_raise_in_the_middle_of_encoding_leaves_the_process_healthy
    values = Array.new(10_000) { |index| "value-#{index}" } << 42

    1_000.times do
      assert_raises(TypeError) { Demo.echo_vec_string(values) }
    end
    GC.start

    assert_equal(["ok"], Demo.echo_vec_string(["ok"]))
  end

  private

  def with_gc_stress
    GC.stress = true
    yield
  ensure
    GC.stress = false
  end
end
