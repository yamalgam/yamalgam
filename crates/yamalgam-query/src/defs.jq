# yg definitions loaded after jaq-core and jaq-std.

def type: _yaml_type;
def keys: keys_unsorted;
def keys_sorted: keys_unsorted | sort;

def totype(p; e): if p then . else fromjson | if p then . else e end end;
def tonumber : totype(isnumber ; error("cannot parse as number" ));
def toboolean: totype(isboolean; error("cannot parse as boolean"));

def transpose: [range([.[] | length] | max) as $i | [.[][$i]]];

def in(xs)    : . as $x | xs | has     ($x);
def inside(xs): . as $x | xs | contains($x);
def  index($i): indices($i)[ 0];
def rindex($i): indices($i)[-1];

def @json: tojson;
