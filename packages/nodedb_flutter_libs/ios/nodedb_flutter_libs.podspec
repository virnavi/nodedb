Pod::Spec.new do |s|
  s.name             = 'nodedb_flutter_libs'
  s.version          = '0.0.2'
  s.summary          = 'Pre-compiled NodeDB native binaries for iOS'
  s.homepage         = 'https://github.com/virnavi/nodedb'
  s.license          = { :type => 'MIT' }
  s.author           = { 'Mohammed Shakib' => 'shakib1989@gmail.com' }
  s.source           = { :path => '.' }
  s.platform         = :ios, '11.0'
  s.swift_version    = '5.0'

  s.vendored_frameworks = 'nodedb.xcframework'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
  }
end
