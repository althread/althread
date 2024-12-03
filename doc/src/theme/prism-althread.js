(function (Prism) {

	var multilineComment = /\/\*(?:[^*/]|\*(?!\/)|\/(?!\*)|<self>)*\*\//.source;
	for (var i = 0; i < 2; i++) {
		// support 4 levels of nested comments
		multilineComment = multilineComment.replace(/<self>/g, function () { return multilineComment; });
	}
	multilineComment = multilineComment.replace(/<self>/g, function () { return /[^\s\S]/.source; });


	Prism.languages.althread = {
		'comment': [
			{
				pattern: RegExp(/(^|[^\\])/.source + multilineComment),
				lookbehind: true,
				greedy: true
			},
			{
				pattern: /(^|[^\\:])\/\/.*/,
				lookbehind: true,
				greedy: true
			}
		],
		'string': {
			pattern: /b?"(?:\\[\s\S]|[^\\"])*"|b?r(#*)"(?:[^"]|"(?!\1))*"\1/,
			greedy: true
		},


		'type-definition': {
			pattern: /\b(?:list|proc|int|float|bool|string)/,
			lookbehind: true,
			alias: 'class-name'
		},

		'function-definition': {
			pattern: /\b(?:program|shared|main|program|always)\b/,
			lookbehind: true,
			alias: 'system-block'
		},

		'keyword': /\b(?:let|const|channel|while|for|in|loop|atomic|if|else|run|send|wait|seq|first|receive)\b/,

        'function': /\b\w+(?=\()/,
		// Hex, oct, bin, dec numbers with visual separators and type suffix
		'number': /\b(?:0x[\dA-Fa-f](?:_?[\dA-Fa-f])*|0o[0-7](?:_?[0-7])*|0b[01](?:_?[01])*|(?:(?:\d(?:_?\d)*)?\.)?\d(?:_?\d)*(?:[Ee][+-]?\d+)?)(?:_?(?:f32|f64|[iu](?:8|16|32|64|size)?))?\b/,
		'boolean': /\b(?:false|true)\b/,
		'punctuation': /->|\.\.=|\.{1,3}|::|[{}[\];(),:]/,
		'operator': /[-+*\/%!^]=?|=[=>]?|&[&=]?|\|[|=]?|<<?=?|>>?=?|[@?]/,        
        'channel-declaration': {
            pattern: /\bchannel\b\s+\w+\s*<\((?:[^,]+,)*[^,]+\)>/,
            inside: {
                'keyword': /\bchannel\b/,
                'punctuation': /[<>()]/,
                'type': /\b(?:bool|int|float|string)\b/,
                'variable': /\w+/
            }
        },
        'attribute-keyword': {
            pattern: /\b(?:seq|first)\b/,
            alias: 'keyword'
        },
        'variable': /\b(?:[a-z]\w*)\b/,
		'constant': /\b[A-Z]\w*\b/,
	};

	Prism.languages.rust['closure-params'].inside.rest = Prism.languages.rust;
	Prism.languages.rust['attribute'].inside['string'] = Prism.languages.rust['string'];

}(Prism));