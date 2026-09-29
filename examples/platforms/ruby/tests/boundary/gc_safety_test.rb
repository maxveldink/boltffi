# frozen_string_literal: true

# The extension holds Ruby objects in C locals while it builds values. These
# tests make the garbage collector run and move objects during those calls,
# so a value that the extension fails to keep alive corrupts a result or
# crashes the process.
class GcSafetyTest < Minitest::Test
  def test_record_classes_survive_compaction
    GC.verify_compaction_references(expand_heap: true, toward: :empty)

    assert_equal(Demo::Point.new(x: 1.0, y: 2.0), Demo.make_point(1.0, 2.0))
    assert_equal(Demo::Person.new(name: "Ada", age: 36), Demo.make_person("Ada", 36))
  end

  private

  def with_gc_stress
    GC.stress = true
    yield
  ensure
    GC.stress = false
  end
end
