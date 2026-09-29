# frozen_string_literal: true

# Ruby benchmark harness.
#
# Each case calls one function of examples/demo through BoltFFI's Ruby C
# extension. When --uniffi-dir names a UniFFI Ruby build of the same crate,
# the case also calls UniFFI's Ruby bindings, which use the ffi gem. Each
# measurement includes one Ruby lambda call per operation, the same for every
# subject.
#
# The harness writes per-sample timings as JSON. The script
# benchmarks/scripts/ruby_bench_to_run.py turns them into benchmark_run.json.

require "json"
require "optparse"
require "rbconfig"

Case = Struct.new(:canonical_name, :build)

module RubyBench
  module_function

  def now_ns
    Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
  end

  def run_loops(callable, loops)
    start = now_ns
    index = 0
    while index < loops
      callable.call
      index += 1
    end
    now_ns - start
  end

  # Warms up the call, then doubles the loop count until one sample takes at
  # least `sample_ns`. Each value is the mean time of one operation in a sample.
  def measure(callable, samples:, sample_ns:, warmup_ns:)
    deadline = now_ns + warmup_ns
    callable.call while now_ns < deadline
    loops = 1
    loops *= 2 while loops < (1 << 30) && run_loops(callable, loops) < sample_ns
    values = Array.new(samples) { run_loops(callable, loops).fdiv(loops) }
    { loops: loops, values_ns: values }
  end

  def fixtures(subject)
    {
      string_small: "hello",
      string_200: "x" * 200,
      string_1k: "x" * 1000,
      string_64k: "x" * 65_536,
      bytes_64k: ("*" * 65_536).b,
      i32_range_1k: (0...1000).to_a,
      i32_range_10k: (0...10_000).to_a,
      i32_vec_1k: subject.generate_i32_vec(1000),
      i32_vec_10k: subject.generate_i32_vec(10_000),
      i32_vec_100k: subject.generate_i32_vec(100_000),
      f64_vec_10k: subject.generate_f64_vec(10_000),
      locations_1k: subject.generate_locations(1000),
      locations_10k: subject.generate_locations(10_000),
      trades_1k: subject.generate_trades(1000),
      trades_10k: subject.generate_trades(10_000),
      particles_1k: subject.generate_particles(1000),
      particles_10k: subject.generate_particles(10_000),
      sensor_readings_1k: subject.generate_sensor_readings(1000),
      sensor_readings_10k: subject.generate_sensor_readings(10_000),
      user_profiles_100: subject.generate_user_profiles(100),
      user_profiles_1k: subject.generate_user_profiles(1000),
      address: subject::Address.new(street: "Market Street", city: "San Francisco", zip: "94103"),
      person: subject::Person.new(name: "Ada", age: 37),
    }
  end

  # BoltFFI keeps the Rust field name `end`, and UniFFI renames it to `_end`.
  def line(subject, key)
    subject::Line.new(start: subject::Point.new(x: 0.0, y: 0.0), key => subject::Point.new(x: 3.0, y: 4.0))
  end

  # Each case builds the callable for one subject from that subject's module
  # `m` and fixtures `f`. The Ruby target does not support enums, classes,
  # callbacks, async functions, or mutable slices, so the catalog cases that
  # need them are not here.
  def cases
    [
      Case.new("noop", ->(m, _f) { -> { m.noop } }),
      Case.new("echo_bool", ->(m, _f) { -> { m.echo_bool(true) } }),
      Case.new("negate_bool", ->(m, _f) { -> { m.negate_bool(true) } }),
      Case.new("echo_i32", ->(m, _f) { -> { m.echo_i32(42) } }),
      Case.new("echo_f64", ->(m, _f) { -> { m.echo_f64(3.14159) } }),
      Case.new("add", ->(m, _f) { -> { m.add(100, 200) } }),
      Case.new("add_f64", ->(m, _f) { -> { m.add_f64(1.25, 2.5) } }),
      Case.new("multiply", ->(m, _f) { -> { m.multiply(2.5, 4.0) } }),
      Case.new("inc_u64_value", ->(m, _f) { -> { m.inc_u64_value(0) } }),
      Case.new("echo_string_small", ->(m, f) { value = f[:string_small]; -> { m.echo_string(value) } }),
      Case.new("echo_string_200", ->(m, f) { value = f[:string_200]; -> { m.echo_string(value) } }),
      Case.new("echo_string_1k", ->(m, f) { value = f[:string_1k]; -> { m.echo_string(value) } }),
      Case.new("echo_string_64k", ->(m, f) { value = f[:string_64k]; -> { m.echo_string(value) } }),
      Case.new("generate_string_1k", ->(m, _f) { -> { m.generate_string(1000) } }),
      Case.new("generate_string_64k", ->(m, _f) { -> { m.generate_string(65_536) } }),
      Case.new("echo_bytes_64k", ->(m, f) { value = f[:bytes_64k]; -> { m.echo_bytes(value) } }),
      Case.new("generate_bytes_64k", ->(m, _f) { -> { m.generate_bytes(65_536) } }),
      Case.new("find_even_100", ->(m, _f) { -> { 100.times { |index| m.find_even(index) } } }),
      Case.new("find_positive_f64", ->(m, _f) { -> { m.find_positive_f64(3.14) } }),
      Case.new("find_name", ->(m, _f) { -> { m.find_name(1) } }),
      Case.new("find_names_100", ->(m, _f) { -> { m.find_names(100) } }),
      Case.new("find_numbers_100", ->(m, _f) { -> { m.find_numbers(100) } }),
      Case.new("roundtrip_locations_100", ->(m, _f) { -> { m.process_locations(m.generate_locations(100)) } }),
      Case.new("roundtrip_i32_vec_1k", ->(m, _f) { -> { m.sum_i32_vec(m.generate_i32_vec(1000)) } }),
      Case.new("echo_vec_i32_10k", ->(m, f) { value = f[:i32_range_10k]; -> { m.echo_vec_i32(value) } }),
      Case.new("find_locations_100", ->(m, _f) { -> { m.find_locations(100) } }),
      Case.new("make_point", ->(m, _f) { -> { m.make_point(3.0, 4.0) } }),
      Case.new("echo_address", ->(m, f) { value = f[:address]; -> { m.echo_address(value) } }),
      Case.new("echo_person", ->(m, f) { value = f[:person]; -> { m.echo_person(value) } }),
      Case.new("echo_line", ->(m, f) { value = f[:line]; -> { m.echo_line(value) } }),
      Case.new("generate_locations_100", ->(m, _f) { -> { m.generate_locations(100) } }),
      Case.new("generate_trades_100", ->(m, _f) { -> { m.generate_trades(100) } }),
      Case.new("generate_particles_100", ->(m, _f) { -> { m.generate_particles(100) } }),
      Case.new("generate_sensor_readings_100", ->(m, _f) { -> { m.generate_sensor_readings(100) } }),
      Case.new("generate_locations_1k", ->(m, _f) { -> { m.generate_locations(1000) } }),
      Case.new("generate_trades_1k", ->(m, _f) { -> { m.generate_trades(1000) } }),
      Case.new("generate_particles_1k", ->(m, _f) { -> { m.generate_particles(1000) } }),
      Case.new("generate_sensor_readings_1k", ->(m, _f) { -> { m.generate_sensor_readings(1000) } }),
      Case.new("generate_locations_10k", ->(m, _f) { -> { m.generate_locations(10_000) } }),
      Case.new("generate_trades_10k", ->(m, _f) { -> { m.generate_trades(10_000) } }),
      Case.new("generate_particles_10k", ->(m, _f) { -> { m.generate_particles(10_000) } }),
      Case.new("generate_sensor_readings_10k", ->(m, _f) { -> { m.generate_sensor_readings(10_000) } }),
      Case.new("sum_ratings_1k", ->(m, f) { value = f[:locations_1k]; -> { m.sum_ratings(value) } }),
      Case.new("sum_trade_volumes_1k", ->(m, f) { value = f[:trades_1k]; -> { m.sum_trade_volumes(value) } }),
      Case.new("sum_particle_masses_1k", ->(m, f) { value = f[:particles_1k]; -> { m.sum_particle_masses(value) } }),
      Case.new("avg_sensor_temp_1k", ->(m, f) { value = f[:sensor_readings_1k]; -> { m.avg_sensor_temperature(value) } }),
      Case.new("process_locations_1k", ->(m, f) { value = f[:locations_1k]; -> { m.process_locations(value) } }),
      Case.new("sum_ratings_10k", ->(m, f) { value = f[:locations_10k]; -> { m.sum_ratings(value) } }),
      Case.new("sum_trade_volumes_10k", ->(m, f) { value = f[:trades_10k]; -> { m.sum_trade_volumes(value) } }),
      Case.new("sum_particle_masses_10k", ->(m, f) { value = f[:particles_10k]; -> { m.sum_particle_masses(value) } }),
      Case.new("avg_sensor_temp_10k", ->(m, f) { value = f[:sensor_readings_10k]; -> { m.avg_sensor_temperature(value) } }),
      Case.new("process_locations_10k", ->(m, f) { value = f[:locations_10k]; -> { m.process_locations(value) } }),
      Case.new("generate_i32_vec_1k", ->(m, _f) { -> { m.generate_i32_vec(1000) } }),
      Case.new("generate_i32_vec_10k", ->(m, _f) { -> { m.generate_i32_vec(10_000) } }),
      Case.new("generate_i32_vec_100k", ->(m, _f) { -> { m.generate_i32_vec(100_000) } }),
      Case.new("sum_i32_vec_1k", ->(m, f) { value = f[:i32_vec_1k]; -> { m.sum_i32_vec(value) } }),
      Case.new("sum_i32_vec_10k", ->(m, f) { value = f[:i32_vec_10k]; -> { m.sum_i32_vec(value) } }),
      Case.new("sum_i32_vec_100k", ->(m, f) { value = f[:i32_vec_100k]; -> { m.sum_i32_vec(value) } }),
      Case.new("generate_f64_vec_10k", ->(m, _f) { -> { m.generate_f64_vec(10_000) } }),
      Case.new("sum_f64_vec_10k", ->(m, f) { value = f[:f64_vec_10k]; -> { m.sum_f64_vec(value) } }),
      Case.new("generate_user_profiles_100", ->(m, _f) { -> { m.generate_user_profiles(100) } }),
      Case.new("sum_user_scores_100", ->(m, f) { value = f[:user_profiles_100]; -> { m.sum_user_scores(value) } }),
      Case.new("count_active_users_100", ->(m, f) { value = f[:user_profiles_100]; -> { m.count_active_users(value) } }),
      Case.new("generate_user_profiles_1k", ->(m, _f) { -> { m.generate_user_profiles(1000) } }),
      Case.new("sum_user_scores_1k", ->(m, f) { value = f[:user_profiles_1k]; -> { m.sum_user_scores(value) } }),
      Case.new("count_active_users_1k", ->(m, f) { value = f[:user_profiles_1k]; -> { m.count_active_users(value) } }),
    ]
  end

  def load_subjects(uniffi_dir)
    require "demo"
    subjects = { "boltffi" => [BenchBoltFFI, :end] }
    if uniffi_dir
      # UniFFI's bindings call `ffi_lib 'demo'`, which resolves from the
      # working directory.
      Dir.chdir(uniffi_dir) { require File.join(uniffi_dir, "demo") }
      subjects["uniffi"] = [Object.const_get(:Demo), :_end]
    end
    subjects
  end

  def run(options)
    subjects = load_subjects(options[:uniffi_dir]).to_h do |name, (subject, line_key)|
      [name, [subject, fixtures(subject).merge(line: line(subject, line_key))]]
    end
    selected = cases.select { |bench| options[:include].nil? || options[:include].match?(bench.canonical_name) }
    abort "no Ruby benchmark cases matched the requested filter" if selected.empty?
    benchmarks = selected.flat_map do |bench|
      subjects.map do |name, (subject, fixtures)|
        callable = bench.build.call(subject, fixtures)
        GC.start
        { name: "#{name}_#{bench.canonical_name}" }.merge(
          measure(callable, samples: options[:samples], sample_ns: options[:sample_ns], warmup_ns: options[:warmup_ns])
        )
      end
    end
    {
      suite: "ruby-bench",
      ruby: {
        engine: RUBY_ENGINE,
        version: RUBY_VERSION,
        description: RUBY_DESCRIPTION,
        executable: RbConfig.ruby,
        yjit: defined?(RubyVM::YJIT) ? RubyVM::YJIT.enabled? : false,
      },
      settings: { samples: options[:samples], sample_ns: options[:sample_ns], warmup_ns: options[:warmup_ns] },
      benchmarks: benchmarks,
    }
  end
end

if $PROGRAM_NAME == __FILE__
  options = { include: nil, output: nil, uniffi_dir: nil, samples: 20, sample_ns: 20_000_000, warmup_ns: 200_000_000 }
  OptionParser.new do |parser|
    parser.on("--output PATH", "Write the raw results to PATH") { |value| options[:output] = value }
    parser.on("--uniffi-dir PATH", "Also run a UniFFI Ruby build in PATH") { |value| options[:uniffi_dir] = File.expand_path(value) }
    parser.on("--include REGEX", "Run only cases whose name matches REGEX") { |value| options[:include] = Regexp.new(value) }
    parser.on("--samples N", Integer, "Samples per case and subject") { |value| options[:samples] = value }
    parser.on("--sample-ms N", Integer, "Minimum time of one sample") { |value| options[:sample_ns] = value * 1_000_000 }
    parser.on("--warmup-ms N", Integer, "Warmup time before sampling") { |value| options[:warmup_ns] = value * 1_000_000 }
  end.parse!
  abort "missing --output" unless options[:output]

  results = RubyBench.run(options)
  File.write(options[:output], JSON.pretty_generate(results) + "\n")
  results[:benchmarks].each do |benchmark|
    values = benchmark[:values_ns].sort
    printf("%-40s %12.1f ns/op\n", benchmark[:name], values[values.size / 2])
  end
end
