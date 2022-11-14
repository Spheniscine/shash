# SHash

SHash is a hash function that is optimized for the use-cases of competitive programming, and, as far as I'm aware, is the first of its kind.

**IMPORTANT NOTICE:** Though this readme will include some analysis of the "security properties" of this function, such claims should be considered on a purely *experimental* basis. I am not a professional cryptographer. As such, until further notice, avoid using this if your hashmaps might process untrusted input from e.g. a network, especially where a DOS (denial of service) attack may impose considerable financial damage.

## Rationale

Existing hash functions basically generally fall into two categories, non-cryptographic that prioritizes speed over security (FNV, FxHash), and generally are "fixed" hash functions that do not admit a "seed" to select a random hash function from a family, and cryptographic that prioritizes security over speed (SHA256, BLAKE3).

Most of the latter tend to be slow enough to make general use in "subroutine" algorithms that aren't related to security or cryptography problematic. There have been several attempts to "bridge" this gap by equipping a non-cryptographic function with a seed, e.g. MurmurHash, however it is quite famously [broken](http://emboss.github.io/blog/2012/12/14/breaking-murmur-hash-flooding-dos-reloaded/), in the sense that there is a way to construct input that collides no matter what seed is chosen.

As a response to these breaks, SipHash was invented - the first cryptographic hash function that was developed to be fast enough to be used in hashmaps. Many programming languages, including Rust, now use a variant of SipHash as their default algorithm for hashmaps.

And the story would end there... except for the unique combination of usage requirements imposed by "competitive programming", a term describing a style of competition where novel algorithmic problems would be revealed to contestants to be solved within a short period of time with a computer program. Examples include [Codeforces](https://codeforces.com/), [AtCoder](https://atcoder.jp/), and [Google Code Jam](https://codingcompetitions.withgoogle.com/codejam).

Such competitions typically impose a runtime limit for each problem, such that a submission is only accepted as correct if all testcases run under that limit. Thus it is very important for any algorithms used to be fast enough. Generally, however, there is no bonus for being faster than the time limit, as contestants are not expected to spend a lot of time on micro-optimizations; the focus is generally on constructing algorithms that scale well with input size, as expressed in "Big O" notation.

However, a measure of security is still needed; a hash function that is too simple (identity hash functions, or some multiplicative hash functions without adequate finalization) might expose regularity in the lower bits for easily defined classes of input, causing entries to be poorly distributed between the hashmap's buckets. More pertinently, Codeforces in particular allows participants to "hack" other contestants' solutions by submitting a test case that is believed to cause it to fail. Thus, a poor hash function can be more directly attacked this way. From a cryptographic point of view, the "hacker" can be considered an adversary.

So, we could use SipHash and be assured that we will never get "hacked", except possibly by some cryptographic genius who should immediately stop participating in such competitions and instead write a paper describing how they found a way to defeat the security claims of SipHash. And most of the time, the time limit is enough to admit such solutions. However, these time limits are set somewhat arbitrarily - the problemsetter is expected to keep the speed of "typical" contest-time implementations in a variety of programming languages in mind to set a "fair" time limit, however different problemsetters have differing ideas on what exactly this means and how much "room" they give. Additionally, due to the large variability of possible implementations, programming styles, and languages, sometimes a problem just turns out to be "tighter" than usual, due either to problemsetter error or due to the fact that the problemsetter was trying to prevent solutions from a higher Big O class from passing, even if implemented using "fast" languages and common constant-factor optimizations such as bitsets. Thus, there is cause for worry when a trial submission using SipHash such as [this example](https://codeforces.com/contest/1654/submission/153433867) takes four seconds out of the allotted five. [This is another example](https://codeforces.com/contest/1045/submission/128055604), which actually failed to meet the time limit.

However, we could note that the adversarial threat arising from these "hacks" is actually significantly weaker than the typical considerations that a cryptographer has to deal with. Generally, real-world applications of cryptography has to protect systems that may be worth thousands of dollars, perhaps even up to billions, thus we have to assume the adversary is willing to spend roughly as much resources to defeat the cryptosystem. However a typical contestant trying to "hack" on Codeforces has to try to code and run a program either locally or through submission that will generate the adversarial test case within contest time limits, and generally has to take time that could be used to solve other problems. Thus they are quite limited both in computing resources, as well as the time needed to modify or customize an algorithm needed to generate a test-case. In more theoretical terms, a cryptographic break that requires resources on the order of 2<sup>40</sup> is considered unacceptable for most purposes, but would be generally sufficient to discourage this class of adversaries.

So, if the security requirements are meaningfully lower, could we perhaps develop and use a hash function that is faster than SipHash but is still strong enough for this use-case?

## Design

Is the MurmurHash break meaningful then, in the light of these lowered security requirements? To try to answer this question, we should examine exactly what the [break](http://emboss.github.io/blog/2012/12/14/breaking-murmur-hash-flooding-dos-reloaded/) is and claims to be. With some thought, a flaw becomes apparent that's almost trivial in hindsight. It is illustrated by this section of the MurmurHash code:

```
uint64_t k = *(uint64_t*)data;

k *= m; // const uint64_t m = 0xc6a4a7935bd1e995;
k ^= k >> r;
k *= m;
```

The issue here is that we are taking raw bits from the input, and applying transforms to it that do not depend on the seed. Thus, an adversary can just apply the inverse of this transform to their input to preserve the collision-causing differentials. In other words, these are *wasted cycles* from an adversarial point of view.

Thus, there is a reasonably efficient "off the shelf" algorithm for generating many seed-independent collisions for MurmurHash. I am currently unaware if this algorithm can be used to meaningfully "hack" another participant's solution - there are several other practical barriers in the way, namely that most problems impose strict constraints on the input, thus generated input that is, for example, too long or not ASCII-compliant might not be accepted as a test-case. However this is worrisome enough, in my opinion, to consider using another algorithm.

Another flaw concerns speed more than security. For each "word" (64 bits) of input, MurmurHash takes three multiplies and some xorshifts to combine it with its current state. After that, it applies two more rounds of multiply + xorshift before outputting. The additional two rounds is called "finalization". The effect is that the graph of input size to speed has a significant "y-intercept", a constant added to the time regardless of the size of an input. Unfortunately, most problems in programming competitions only require using single 32-bit or 64-bit integers as keys, so finalization takes a significant portion of the time needed to hash. Note that SipHash shares this "flaw", which is apparent in the naming of the variant used in Rust, SipHash-1-3. It means "1 round for compression, 3 rounds for finalization". This means that the typical single-word input from a competitive programming problem would have four to five SipHash rounds applied to it (the additional round because of length padding). 

These finalizers are applied in order to improve diffusion and the "avalanche property" in the final output, so that distribution between hash buckets more closely approximates theoretical randomness. Most arbitrary-length hash functions published use some form of finalizer. However, due to the high frequency of single-word inputs in our use-case, when designing a hash function for it, we should ideally choose a fast compression function that already comes with good diffusion and having either a trivial or no finalizer, thus, when a hashmap with keys that consist only of single words (like machine integers) is compiled, we just get that compression function inlined in the assembly code.

With this is mind we can examine the design of the heart of the SHash implementation in this repository, [the code in `SHash::write_u64`](https://docs.rs/shash/0.1.0/src/shash/lib.rs.html#77-86):

```rust
pub struct SHash(u64, u128);
impl SHash {
    #[inline] fn write_u64(&mut self, i: u64) {
        let mut z = i.wrapping_add((self.1 >> 64) as u64);
        z ^= z.rotate_right(25) ^ z.rotate_right(47);
        z = z.wrapping_mul(0x9E6C63D0676A9A99).wrapping_add(self.0);
        z ^= z >> 23 ^ z >> 51;
        z = z.wrapping_mul(0x9E6D62D06F6A9A9B);
        z ^= z >> 23 ^ z >> 51;
        self.0 = z;
        self.1 = self.1.wrapping_mul(0xda942042e4dd58b5);
    }
}
```

The SHash struct consists of two variables. The first variable (`self.0`) is the 64-bit **state**. The second variable (`self.1`) is a 128-bit **dither**.

The purpose of the dither is actually apparent from the *last* line of this code - this is actually a [Lehmer64](https://lemire.me/blog/2019/03/19/the-fastest-conventional-random-number-generator-that-can-pass-big-crush/) RNG. Additionally, it is never affected by the input.

The first line adds the high-half of the dither (the output of the RNG) to the input bits. By combining this psuedorandom string with the input *before* any transformations are applied, we avoid the seed-independent collision flaw in MurmurHash, or at least hopefully make them much harder to find.

The rest of the transformations on `z` is a compression function based on [NASAM](http://mostlymangling.blogspot.com/2020/01/nasam-not-another-strange-acronym-mixer.html). Basically, input + dither goes through some bitmixing before being added to the state (which had been initialized randomly too), which then undergoes more mixing. As NASAM has better statistical randomness properties (as described by the test procedure on the NASAM homepage) than either MurmurHash3's finalizer or Splitmix/Variant13, this eliminates the need for separate compression and finalization steps, solving the second problem.

Note that in the case of single-word keys, the RNG stepping is actually optimized out. So the compiled hashing code reduces to a single round of NASAM with two additions of randomized seed values.

[This submission](https://codeforces.com/contest/1654/submission/153455964) is for the same problem as the [previous example](https://codeforces.com/contest/1654/submission/153433867). Using SHash instead of the default SipHash-1-3 allowed this problem to be solved in two seconds instead of four seconds of runtime.

## Security and preliminary cryptanalysis

If we knew the seed, we could craft input that combines with the pseudorandom string from the dither such that it will zero out the bits in the state and keep it at zero to generate many collisions. However, if the seed is different, the dither will likely generate a completely different pseudorandom string, thus our collisions will not work on this different seed.

Note that this function is quite trivially insecure in cases where the output is exposed. This is because all the NASAM operations can be inverted, exposing the seed-derived differentials sufficiently that they can be solved for. This is not a factor in competitive programming, as hash values aren't typically exposed. Even if it was, a "hacker" cannot typically change their input based on the knowledge of the hash values or seed from a particular run.
