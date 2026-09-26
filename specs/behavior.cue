package spec

#U32: int & >=0 & <=4294967295

#Run: {
	input!: {
		a!: #U32
		b!: #U32
	}
	// Contrat limité aux entrées dont la somme tient dans un u32.
	output!: #U32 & (input.a + input.b)
}

// Exige au moins une observation.
runs!: [#Run, ...#Run]
