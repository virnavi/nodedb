Pod::Spec.new do |s|
  s.name             = 'nodedb_flutter_libs'
  s.version          = '0.0.1'
  s.summary          = 'Pre-compiled NodeDB native binaries for macOS'
  s.homepage         = 'https://github.com/virnavi/nodedb'
  s.license          = { :type => 'MIT' }
  s.author           = { 'Mohammed Shakib' => 'shakib1989@gmail.com' }
  s.source           = { :path => '.' }
  s.platform         = :osx, '10.15'

  s.vendored_libraries = 'libnodedb_ffi.dylib'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'LD_RUNPATH_SEARCH_PATHS' => '$(inherited) @executable_path/../Frameworks',
  }
end
