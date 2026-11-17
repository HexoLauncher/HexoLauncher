const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

interface Sprite {
	palette: string[];
	rows: string[];
}

const SPRITES: Record<string, Sprite> = {
	villages: {
		palette: [
			'#5f3523',
			'#835b4a',
			'#402114',
			'#33190e',
			'#b4652a',
			'#d39632',
			'#f5cc27',
			'#fed639',
			'#ffec4f',
			'#9c4f16',
			'#fff68d',
			'#feffbd'
		],
		rows: [
			'................',
			'..ABAACCAAACAC..',
			'..CACDDDCCCDCD..',
			'.....EFFFFE.....',
			'....EGHIIHFJ....',
			'....FHIKIIHJ....',
			'....FHKKIIHJ....',
			'....FHKIIKHJ....',
			'....EHIKKIHJ....',
			'....FKLKKLKJ....',
			'....EFHHHHFJ....',
			'...FKLLLLKKKE...',
			'...EHIHHHIIFJ...',
			'....JJJJJJJJ....',
			'................',
			'................'
		]
	},
	desert_pyramids: {
		palette: ['#edebcb', '#e7e4bb', '#e3dbb0', '#dad2a3', '#d5c496', '#d1ba8a', '#c6ae71'],
		rows: [
			'AAAAABAABBBABBBB',
			'CDDDDEEDDDDDDDDE',
			'EFEFFFFFFFFGFGGG',
			'AAAAAABBBBBBBBBG',
			'ADCDDGGDDGGDDEDG',
			'BDDDDFGDDFGDDEEG',
			'CDCCDCCGGCCDDDEG',
			'BDCDDDGGGGDDDEEG',
			'CCDDCCGGCCDCDEEG',
			'EFFFFFFFFFFGGGGG',
			'BBBBBBBBBBBBCBCG',
			'BDDGDDGGDDDGGGCG',
			'CDGEFFEFFEFCCGEG',
			'CFCFCDCFCDFDDFDG',
			'CDCCDDDDEEDDEDEG',
			'FFFFFFFFFGFGGGGG'
		]
	},
	jungle_temples: {
		palette: [
			'#627941',
			'#525d39',
			'#a6a6a6',
			'#5a6d41',
			'#7a8f55',
			'#738552',
			'#616161',
			'#888788',
			'#b5b5b5',
			'#6e6d6d',
			'#525252'
		],
		rows: [
			'ABCCDBBDEFAGHICF',
			'BCIHHBDEFAHBGHJH',
			'GCHCJBFAADJBBBHJ',
			'GGHJDBHDHJBAFABB',
			'KJGBBHGDGGHFFFAD',
			'IBEFHCJGKHCAHADH',
			'CHBACHHJDBBHHJBC',
			'HHBBBHBFFJABJHGH',
			'JBBEFAABAHFADGKJ',
			'BDFDAAJHHJDABDAB',
			'HJAADBBBBJJDBAFC',
			'HBBDBDAEFBGBHCCH',
			'BDEFJBEFAFJGHHJH',
			'AEAHHBFAJHHJBBHJ',
			'HFHJHJBJHHJGBHAB',
			'BHGGJDBBBGGHICHA'
		]
	},
	igloos: {
		palette: ['#f7fefe', '#ffffff', '#f0fdfd'],
		rows: [
			'ABBBBAABABBBBAAA',
			'BBBBBBACCBBBABAB',
			'BBBBACCCCBBCBBBB',
			'BBACCCCBBBCCABBB',
			'BBCCCAABBCCABBBB',
			'AACCABABBACABBAB',
			'ABBAACBBABABBBCA',
			'BBABCCBBAABBBCCC',
			'BBBACBABACBBACCB',
			'BACCABCCCAABCCAB',
			'ABCCBBBCCABCBAAA',
			'BABBBBACCACCABBA',
			'BBBBAACCAAAABBBB',
			'BBBAACCAAABABBCC',
			'ABBCCCAAABBBBCCA',
			'AAABAAAABBBBCCAA'
		]
	},
	swamp_huts: {
		palette: [
			'#494848',
			'#2d2d32',
			'#1b1b21',
			'#595858',
			'#343438',
			'#3f3e42',
			'#676161',
			'#4f4f4f'
		],
		rows: [
			'................',
			'................',
			'................',
			'.....AAAAA......',
			'...AABCCCBAA....',
			'..DACCCCCCCBE...',
			'..EDDBCCCBAFB...',
			'..AEGGAAAHEBB...',
			'..ADHEEEFEFFB...',
			'..ADGGBFFHFFB...',
			'..EHGGDHAFFBB...',
			'..ABFHHAFFEBA...',
			'..B.BBFDBBB.B...',
			'......BAB.......',
			'.......E........',
			'................'
		]
	},
	pillager_outposts: {
		palette: [
			'#4d2e11',
			'#2a1a09',
			'#6d4218',
			'#979797',
			'#6d6d6d',
			'#90581f',
			'#444444',
			'#b5732f',
			'#3b240d',
			'#1f1206',
			'#969696',
			'#482b10',
			'#36200b'
		],
		rows: [
			'............A...',
			'........AAABCA..',
			'...DE.AFFCCCAA..',
			'..D..GHIIIB.G...',
			'..E.AHIJ...G....',
			'...GFIAAJ.E.....',
			'..AFIAACAK......',
			'..CLJAAKACJ.....',
			'.AFB.JCAKAAJ....',
			'.AFB..ACAKCAJ...',
			'.AC..E.JACACM...',
			'.BF.G...JACACJ..',
			'ACAG.....JMCAMJ.',
			'.BB........JMACJ',
			'............JCJJ',
			'.............JJ.'
		]
	},
	ocean_monuments: {
		palette: [
			'#53887c',
			'#508678',
			'#598b7f',
			'#4f8376',
			'#52887b',
			'#508578',
			'#568a7d',
			'#4f8576',
			'#74ac9f',
			'#90bcb0',
			'#81b4a7',
			'#b1d4ca',
			'#a7c8bf',
			'#c6dfd8',
			'#c0d8d3',
			'#afcfc7',
			'#91bfb4',
			'#8fbeb1',
			'#c4ddd6',
			'#d3e2dc',
			'#d3e3dd',
			'#d6e6e0',
			'#d5e5df',
			'#d4e4de',
			'#d6e4de',
			'#cfe2dc',
			'#cae1da',
			'#dce8e3',
			'#dfeae4',
			'#e0eae4',
			'#e1ebe6',
			'#dee9e3',
			'#d8e7df',
			'#4a7f72',
			'#dee9dc',
			'#dfe9dc',
			'#dde8dc',
			'#b6d7cd',
			'#4c8276',
			'#dfeade',
			'#e4ece4',
			'#e4ede4',
			'#e0ebdf',
			'#e5ede5',
			'#dfe9de',
			'#accdc6',
			'#e3ebe4',
			'#e2eae4',
			'#4a8073',
			'#54897d',
			'#c9dfd9',
			'#e0e9e2',
			'#e3ece4',
			'#53867a',
			'#dee8dd',
			'#deeae1',
			'#498175',
			'#497e71'
		],
		rows: [
			'ABBCDAEFCEBGFFCC',
			'GHIJKLMNOPPLQIBH',
			'DIRSTUVWVXYZaJIH',
			'DQNZWbcddefgZaJh',
			'GMZYcijikjkfYZlm',
			'CPXfnjopqrjscXtD',
			'HPgfjpouvrrncWtw',
			'xyYfirzzvu0ieWSx',
			'Cagdsv0esv0qeWOA',
			'1lYf2odvpurkfVtA',
			'CMZbk2oruoiscULw',
			'Gtygbsiinj23bYt4',
			'FQONVbddfcfbTSJh',
			'DIJSNXXUWUXYOQIh',
			'D1IQKKLSNlPlRIE1',
			'Ah5hHw4wwm5AxwxF'
		]
	},
	woodland_mansions: {
		palette: [
			'#a05b23',
			'#eadb84',
			'#85400f',
			'#fdfbee',
			'#f8eea5',
			'#d1a75d',
			'#41e67f',
			'#00bb46',
			'#c58742'
		],
		rows: [
			'................',
			'.....AAAAAA.....',
			'....ABBBBBBA....',
			'....CBDDEEBC....',
			'....CFFEBFFC....',
			'....CDGFFDGC....',
			'....CGHDEGHC....',
			'....CIEBFFIC....',
			'.AAAACIIBBCAAAA.',
			'.ABFCBIIFFICIFC.',
			'..CCAEDEBBFACC..',
			'....ABFFFFIC....',
			'....CAAAAAAC....',
			'.....CFIIIC.....',
			'.....CFIIFC.....',
			'......CCCC......'
		]
	},
	ancient_cities: {
		palette: ['#052a32', '#034150', '#0a5060', '#111b21', '#009295', '#29dfeb', '#0d1217'],
		rows: [
			'................',
			'..AA............',
			'.ABCDD..........',
			'.ACCECD.........',
			'.ABEFECD........',
			'.DADECBAD.......',
			'.DADDABBAD......',
			'..DADDCBBAG.....',
			'...GCDBBECBG....',
			'....GABDBDABG...',
			'.....GADDAAAAG..',
			'......GBDDBACG..',
			'.......GCDDCECG.',
			'........GEBACBG.',
			'.........GGGGG..',
			'................'
		]
	},
	trial_chambers: {
		palette: [
			'#454a42',
			'#292828',
			'#525a51',
			'#a19e94',
			'#85837a',
			'#6c716b',
			'#ff9951',
			'#373b35',
			'#84432d',
			'#9a5038',
			'#54281a',
			'#6d3421',
			'#c15a36',
			'#d67b5b',
			'#fc9982'
		],
		rows: [
			'.....AAAAAB.....',
			'....ACDDEFCB....',
			'....AEFFFFFB....',
			'....AFGCCGCB....',
			'....AFCDFCFB....',
			'.....HCHHCB.....',
			'.....BHIIHB.....',
			'.....HJIBBH.....',
			'......KLJJ......',
			'......MJKL......',
			'......LNMK......',
			'......LOMJLL....',
			'......LOONNK....',
			'......LONKK.....',
			'......LNNNML....',
			'......KKKKKK....'
		]
	},
	trail_ruins: {
		palette: [
			'#754236',
			'#9f5949',
			'#693b31',
			'#905041',
			'#5e372f',
			'#83493b',
			'#522d25',
			'#47251d'
		],
		rows: [
			'................',
			'.......AAAAAAA..',
			'......ABBBBBBC..',
			'..AAAABBBDDDDE..',
			'..CBBBDFFFFFFE..',
			'..EBEEEEFFFFAE..',
			'..EBFGFGEFFFAG..',
			'..GBFHAGEFFGAG..',
			'..GEEGGGEEGGEG..',
			'..GHHHAHGAFHAAG.',
			'....GHFFGHFAAAH.',
			'...ABHFGHFAEEEH.',
			'..ABEHHHEEEEEHH.',
			'.GDCEEEEEEHHH...',
			'.HHHHHHHHH......',
			'................'
		]
	},
	ruined_portals: {
		palette: [
			'#06030b',
			'#000001',
			'#2b0178',
			'#100c1c',
			'#3b2754',
			'#271e3d',
			'#6006ab',
			'#8308e4'
		],
		rows: [
			'ABBBCADABEFAAFEF',
			'ABADBCCBEDAGFEDA',
			'BBEEFBAFFDDHDFBA',
			'CEFDDFGDADHBAGCB',
			'BFCDCCGGHHBAAAGA',
			'DDHGBDAAAGEFAACB',
			'GHADFEECABFBGBBC',
			'ABDEDDGCBHBGBEFB',
			'BBEFCHCDFFGBFDDD',
			'CDGBDHFDAGGDADGC',
			'DCCGHAACAABBDGBF',
			'BDACDGADAEEFDDCC',
			'AEEDBCDDFEGADCDA',
			'FFDAGABDDDGCCBFA',
			'BFAGDAFAAGDBCBBA',
			'DABGBADFDCAAACAA'
		]
	},
	shipwrecks: {
		palette: [
			'#6f5a38',
			'#473a24',
			'#c5a678',
			'#59482d',
			'#bc9862',
			'#9f844d',
			'#836c3d',
			'#39241b'
		],
		rows: [
			'................',
			'....AAB.........',
			'....ACBDDDDD....',
			'...AEDDFFEEAD...',
			'..ADDFFBBBBEAD..',
			'ADDFFBBDDAADED..',
			'DFFBBDBCDGGAEB..',
			'BCBDDDBEDBGGEB..',
			'BFCDAGEAABDFDH..',
			'BFECGGAABGEDED..',
			'BFEECGGCEABGFED.',
			'.HEEECCFFBEFGFED',
			'..HEECFFAHBEFGED',
			'...HFCGHH..BEEGD',
			'....HHH.....BBBB',
			'................'
		]
	},
	ocean_ruins: {
		palette: [
			'#a6846a',
			'#baad96',
			'#ca7548',
			'#d48080',
			'#d4ccc3',
			'#ae4635',
			'#6d533f',
			'#8a6a53',
			'#80411f',
			'#413c2d'
		],
		rows: [
			'................',
			'.....AAAAA......',
			'...AABCDEEAA....',
			'..ABEEEEDEEBA...',
			'.ABDEEEBAABCFA..',
			'.AEEDCBABBABBG..',
			'.AEEEEABGHBABAG.',
			'.HDEEBBBGHAHFAG.',
			'.HBCCFAAHGAHBIG.',
			'.GHBEEBAABHBBAG.',
			'..GHABBHHHBFAG..',
			'...GJJJEDEBAIG..',
			'..GJJJGECBAHG...',
			'..GHGGHFAAGG....',
			'...GAAGGGG......',
			'................'
		]
	},
	abandoned_camp: {
		palette: [
			'#b13f00',
			'#c96c03',
			'#dfa21b',
			'#efcd56',
			'#f9ebab',
			'#4c3d26',
			'#fefefb',
			'#c29d62',
			'#b8945f',
			'#362b1b',
			'#9f844d',
			'#5f4a2b',
			'#967441',
			'#745a36',
			'#917142',
			'#987849'
		],
		rows: [
			'................',
			'........A.......',
			'.......AAA......',
			'.......BBA......',
			'.....A.CBBA.....',
			'.....ABCCBA.....',
			'....ABCDDCBA....',
			'....BCDEDDCB....',
			'...FABDCEGBAF...',
			'..FHIHGGBBHIHJ..',
			'.FFHKIAGEAHKIJJ.',
			'.FLIIMLNNOIIMLJ.',
			'.FNLFFNOONLFFNJ.',
			'.FPPNNONLLLNNOJ.',
			'.JJJJJJJJJJJJJJ.',
			'................'
		]
	},
	buried_treasures: {
		palette: [
			'#1d6d8d',
			'#2180a1',
			'#1f96b1',
			'#0d485f',
			'#1dc2d1',
			'#0b3a4d',
			'#0f5875',
			'#082054'
		],
		rows: [
			'................',
			'......AAAA......',
			'....AABCCBAA....',
			'...ABCCCCCCBD...',
			'..ABCEECCCCBBD..',
			'..ABCEECCCCBAF..',
			'.ABCCCCCCCBBAGF.',
			'.ABBCCCBBCBBAAF.',
			'.FGABBBBBBBAAGH.',
			'.FFFDGAAAAGDHHF.',
			'..FDFFFFFHHHDH..',
			'..FDDGGGGGGDFH..',
			'...FDDGGGGDFH...',
			'....HHDFFFHH....',
			'......HHHH......',
			'................'
		]
	},
	mineshafts: {
		palette: [
			'#373127',
			'#7f5f22',
			'#a76e1f',
			'#8f691d',
			'#ab792d',
			'#281e0b',
			'#3f4243',
			'#684e1e',
			'#767676',
			'#232c2f',
			'#cdcdcd',
			'#a5a5a5',
			'#949798',
			'#808389',
			'#676f73',
			'#a9aeaf',
			'#6b6e72',
			'#9ea4a6',
			'#09090a'
		],
		rows: [
			'.......AAA......',
			'.....AABCBA.....',
			'...AABCDDCCA....',
			'..ADCECCDECDA...',
			'..AFCEECCEECDF..',
			'.GFHICCDCCCDFFJ.',
			'.GGHKLCDDDFFHJJ.',
			'.GMGILDFFFHJJNJ.',
			'.GOPGIJHFHJNPQJ.',
			'.GMNPGJJPHNPONJ.',
			'.JOPNMGQRNPONOJ.',
			'..JOPOJPQRONGS..',
			'...JOMJQNQJJSJ..',
			'....JOJNGS......',
			'.....JJJSJ......',
			'................'
		]
	},
	nether_complexes: {
		palette: [
			'#30181c',
			'#3e1e24',
			'#46262c',
			'#4b2a2e',
			'#552e34',
			'#49242a',
			'#1e1013',
			'#371920'
		],
		rows: [
			'................',
			'................',
			'..........AA....',
			'.......AAABCA...',
			'....AAABCCDCCA..',
			'.AAABCCCCCCCCCA.',
			'AEDCDDCCCDCDEEBA',
			'AFECCCCDECEEBBBG',
			'AFFECDHEEEBBBHBG',
			'AFFFEEBBBBBBBBHG',
			'ABFFEBHHBBBBHGG.',
			'.ABFEBBBHHGGG...',
			'..ABEBHGGG......',
			'...AAGG.........',
			'................',
			'................'
		]
	},
	nether_fossils: {
		palette: ['#cbc6a5', '#e8e5d2', '#fcfbed', '#7b7e6b', '#fffffd'],
		rows: [
			'................',
			'...........ABA..',
			'..........ABCDD.',
			'..........ACECBD',
			'..........ACCBAD',
			'.........ACCDDD.',
			'........ACCD....',
			'......ACC.D.....',
			'......DACC......',
			'....AACD.C......',
			'..BBEECD........',
			'.BCEEED.........',
			'.ABAEBD.........',
			'..ADBAD.........',
			'....DD..........',
			'................'
		]
	},
	end_cities: {
		palette: [
			'#4b4b4b',
			'#353535',
			'#737373',
			'#7f7f98',
			'#8f8fb3',
			'#8c8c8c',
			'#706e8d',
			'#696969'
		],
		rows: [
			'................',
			'................',
			'....AAABBAAA....',
			'..AACDAAAADCAA..',
			'.ACDEEB..AEEDCA.',
			'.ADFFEB..AEFFDB.',
			'.AFFDDB..ADDFFB.',
			'.ACDDG..ABGDDCB.',
			'.ADCCGAB..GCCDB.',
			'.AHCGG..ABGGCAB.',
			'..AHGCB..BCGHB..',
			'..BHGHB..BHGHB..',
			'...BHB....BHB...',
			'...BB......BB...',
			'................',
			'................'
		]
	},
	strongholds: {
		palette: [
			'#316364',
			'#1e4835',
			'#4e8386',
			'#659b7d',
			'#cbfcdd',
			'#b5e45a',
			'#88c35a',
			'#71ac49',
			'#97d16b',
			'#579143',
			'#102c31',
			'#1b2b10'
		],
		rows: [
			'................',
			'................',
			'.......ABB......',
			'.....AACCCBB....',
			'....ACDDDDDAB...',
			'...BCDEFGHDDAB..',
			'...BDEFIBJHDDK..',
			'..BADFBLBIJHDAK.',
			'..BAHHBGBLJHDAK.',
			'..BADHBLBJGHHAK.',
			'...BCDHGBIHCDK..',
			'...BACDHJHCHAK..',
			'....KACCCCDAK...',
			'.....KKAAAKK....',
			'.......KKK......',
			'................'
		]
	}
};

const SIZE = 16;
const DRAW = 28;
const images = new Map<string, HTMLImageElement>();

function svgOf(set: string, rim: string) {
	const sprite = SPRITES[set];
	if (!sprite) return '';
	const colorAt = (x: number, y: number) => {
		if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) return '';
		const ch = sprite.rows[y][x];
		return ch === '.' ? '' : sprite.palette[ALPHABET.indexOf(ch)];
	};
	const rimAt = (x: number, y: number) =>
		!colorAt(x, y) &&
		(colorAt(x - 1, y) || colorAt(x + 1, y) || colorAt(x, y - 1) || colorAt(x, y + 1))
			? rim
			: '';
	const at = (x: number, y: number) => colorAt(x, y) || rimAt(x, y);
	let body = '';
	for (let y = -1; y <= SIZE; y++) {
		let x = -1;
		while (x <= SIZE) {
			const color = at(x, y);
			if (!color) {
				x++;
				continue;
			}
			let end = x + 1;
			while (end <= SIZE && at(end, y) === color) end++;
			body += `<rect x="${x + 1}" y="${y + 1}" width="${end - x}" height="1" fill="${color}"/>`;
			x = end;
		}
	}
	return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${SIZE + 2} ${SIZE + 2}" width="${DRAW}" height="${DRAW}" shape-rendering="crispEdges">${body}</svg>`;
}

export function structureSpriteUrl(set: string, rim: string) {
	const svg = svgOf(set, rim);
	return svg ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}` : '';
}

export function structureSprite(set: string, rim: string, onLoad?: () => void) {
	const key = `${set}|${rim}`;
	let img = images.get(key);
	if (!img) {
		const url = structureSpriteUrl(set, rim);
		if (!url) return null;
		img = new Image();
		img.onload = () => onLoad?.();
		img.src = url;
		images.set(key, img);
	}
	return img.complete && img.naturalWidth > 0 ? img : null;
}
