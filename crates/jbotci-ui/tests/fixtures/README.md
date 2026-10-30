# Float16 reference

`f16-reference.json` records exact input and output bits for IEEE binary16 conversion.
Python `struct` first reads each float32 input bit pattern with `<f`, then writes the reference result with `<e`.
The cases cover both signs, halfway values and their float32 neighbors, subnormal boundaries, overflow, and a fixed random sample.
The generator maps Python overflow exceptions to signed infinity.
NaN payloads have no fixed representation in this fixture.
