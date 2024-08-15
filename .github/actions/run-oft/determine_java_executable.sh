#!/bin/bash

# try to find most recent JDK that is (usually) available on ubuntu-latest runner
if [[ -n ${JAVA_HOME_21_X64} && -d ${JAVA_HOME_21_X64}/bin ]]; then
  java_home_bin=${JAVA_HOME_21_X64}/bin
elif [[ -n ${JAVA_HOME_17_X64} && -d ${JAVA_HOME_17_X64}/bin ]]; then
  java_home_bin=${JAVA_HOME_17_X64}/bin
else
  # fall back to whatever is on the path
  javap_path=$(which javap)
  java_home_bin=$(dirname "${javap_path}")
fi

javap_executable=${java_home_bin}/javap

function get_class_file_format_version() {
  class_name=$1
  ${javap_executable} -verbose -cp "${CLASSPATH}" "${class_name}" | grep "major version" | cut -d " " -f5
}

# determine if the installed JRE is sufficient for running OFT
if [[ -x ${javap_executable} ]]; then
  installed_jre_class_file_format_version=$(get_class_file_format_version java.lang.String)
  echo "Installed JRE supports class file format version ${installed_jre_class_file_format_version}"

  oft_core_class_file_format_version=$(get_class_file_format_version org.itsallcode.openfasttrace.core.cli.CliStarter)
  asciidoc_plugin_class_file_format_version=$(get_class_file_format_version org.itsallcode.openfasttrace.importer.asciidoc.AsciiDocImporter)

  # determine the minimum class file format version needed for running OFT
  if [[ ${oft_core_class_file_format_version} -ge ${asciidoc_plugin_class_file_format_version} ]]; then
    minimum_class_file_format_version=${oft_core_class_file_format_version}
  else
    minimum_class_file_format_version=${asciidoc_plugin_class_file_format_version}
  fi
  echo "OpenFastTrace requires JRE supporting class file format version ${minimum_class_file_format_version}"

  # check if the installed JRE is sufficient
  if [[ ${installed_jre_class_file_format_version} -ge ${minimum_class_file_format_version} ]]; then
    java_executable=${java_home_bin}/java
    if [[ -x ${java_executable} ]]; then
      echo "Using installed JRE (${java_executable}) for running OpenFastTrace"
      echo "JAVA_CMD=${java_executable}" >> "$GITHUB_ENV"
    else
      echo "could not find java command (${java_executable})"
    fi
  else
    echo "OpenFastTrace cannot be run with installed JRE."
    echo "JRE only supports class file format <= ${installed_jre_class_file_format_version} but OFT requires class file format ${minimum_class_file_format_version}."
  fi

else
  echo "OpenFastTrace requires a JRE to be installed"
fi
