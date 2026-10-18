package spec

#U32Max: 4294967295
#U32:    int & >=0 & <=#U32Max

#Run: {
	input!: {
		a!: #U32
		b!: #U32
	}
	// Only covers inputs whose sum fits in a u32.
	output!: #U32 & (input.a + input.b)
}

// At least one observation is required, so an empty file cannot pass.
runs!: [#Run, ...#Run]
