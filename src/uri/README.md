# uProtocol URI

## Overview

The following folder contains the data model, factory, and validators to implement [uProtocol URI Specifications](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc)

Matches the uProtocol URI Format. and is used to define source and sink (destination) attributes of uProtocol.
The factory builds URIs.

URI is used as a method to uniquely identify devices, services, and resources on the network.

## UUri

__An Uri is built from the following elements:__

* __UAuthority__ - represents the device and domain of the software, the deployment. You can specify local or remote options.
* __UEntity__ - The Software Entity defines the software name and version.
* __UResource__ - The resource of the software can be a service name, and instance in the service and the name of the protobuf IDL message.

### UAuthority

An Authority consists of a device and a domain per uProtocol URI format.

An Authority represents the deployment location of a specific Software Entity.

### UEntity - uE

An Software Entity is a piece of software deployed somewhere on a device. The uE is used in the source and sink parts of communicating software.

A uE that *publishes* events is a *Service* role.

A uE that *consumes* events is an *Application* role.

A uE may combine bother Service and Application roles.

### UResource

A service API - defined in the uE - has Resources and Methods. Both of these are represented by the UResource class.

An UResource is something that can be manipulated/controlled/exposed by a service.

Resources are unique when prepended with UAuthority that represents the device and Software Entity that represents the service.

An Resource represents a resource from a Service such as "door" and an optional specific instance such as "front_left".
In addition, it can optionally contain the name of the resource Message type, such as "Door".

The Message type matches the protobuf service IDL that defines structured data types. A message is a data structure type used to define data that is passed in events and rpc methods.
