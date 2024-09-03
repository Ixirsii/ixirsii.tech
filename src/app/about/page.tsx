import Image from "next/image";
import Link from "next/link";

export default function About() {
  return (
    <>
      <h1>About</h1>

      <p>
        Ryan (Ixirsii) is a software developer, gamer, and music enthusiast. 
        He has a Bachelor of Science in Computer Science from the University
        of North Texas. Most of Ryan's professional experience is in Java,
        writing APIs that interface with NoSQL databases but he also has
        experience in Kotlin, Rust, and SQL databases on the backend, as well
        as JavaScript and TypeScript experience on the frontend.
      </p>

      <p>
        In his free time Ryan enjoys playing video games and listening to music.
        Ryan played drums in high school and college, and would like to learn
        guitar. Ryan primarily plays competitive online games, but his favorite
        game is the Mass Effect series.
      </p>

      <hr />

      <div className="divide-x divide-dashed divide-teal-800 flex flex-row justify-center">
        <div className="flex-1 text-right">
          <h2 className="mr-4">Podcasts</h2>

          <div className="bg-teal-800/10 hover:bg-teal-800/20 flex flex-row py-4 rounded-xl">
            <Link
              className="mx-4"
              href="https://www.deliberatesound.org/"
              target="_blank">
                <Image
                  src="/images/Deliberate Sound Logo.png"
                  alt="Deliberate Sound"
                  style={{ objectFit: "contain" }}
                  width={128}
                  height={128}
                />
            </Link>
            <div className="flex-1">
              <Link href="https://www.deliberatesound.org/" target="_blank">
                <h3 className="mr-4">Deliberate Sound</h3>
              </Link>
              <div className="links">
                <Link
                  href="https://www.deliberatesound.org/our-lie-in-april"
                  target="_blank">
                    Our Lie In April
                    <Image
                      src="/images/Our Lie In April Logo.png"
                      alt="Our Lie In April"
                      width="24"
                      height="24"
                    />
                </Link>
              </div>
            </div>
          </div>
        </div>

        <div className="flex-1">
          <h2 className="ml-4">Links</h2>

          <div className="links w-fit">
            <Link href="https://anilist.co/user/Ixirsii" target="_blank">
              <Image
                src="/images/AniList Logo.svg"
                alt="AniList"
                width="24"
                height="24"
              />
              <span>Anilist</span>
            </Link>
            <Link href="https://bitbucket.org/Ixirsii/" target="_blank">
              <Image
                src="/images/BitBucket Logo.svg"
                alt="BitBucket"
                width="24"
                height="24"
              />
              BitBucket
            </Link>
            <Link href="https://bsky.app/profile/ixirsii.tech" target="_blank">
              <Image
                src="/images/Bluesky Logo.svg"
                alt="Bluesky"
                width="24"
                height="24"
              />
              Bluesky
            </Link>
            <Link href="https://discord.gg/KTgshSbEUU" target="_blank">
              <Image
                src="/images/Discord Logo.svg"
                alt="Discord"
                width="24"
                height="24"
              />
              Discord
            </Link>
            <Link href="https://github.com/Ixirsii" target="_blank">
              <Image
                src="/images/GitHub Logo.svg"
                alt="GitHub"
                width="24"
                height="24"
              />
              GitHub
            </Link>
            <Link href="https://www.instagram.com/ixirsii/" target="_blank">
              <Image
                src="/images/Instagram Logo.svg"
                alt="Instagram"
                width="24"
                height="24"
              />
              Instagram
            </Link>
            <Link href="https://mastodon.social/@ixirsii" target="_blank">
              <Image
                src="/images/Mastodon Logo.svg"
                alt="Mastodon"
                width="24"
                height="24"
              />
              Mastodon
            </Link>
            <Link href="https://open.spotify.com/user/shadowhawk54" target="_blank">
              <Image
                src="/images/Spotify Logo.svg"
                alt="Spotify"
                width="24"
                height="24"
              />
              Spotify
            </Link>
            <Link href="https://www.threads.net/@ixirsii" target="_blank">
              <Image
                src="/images/Threads Logo.svg"
                alt="Threads"
                width="24"
                height="24"
              />
              Threads
            </Link>
            <Link href="https://www.twitch.tv/ixirsii" target="_blank">
              <Image
                src="/images/Twitch Logo.svg"
                alt="Twitch"
                width="24"
                height="24"
              />
              Twitch
            </Link>
            <Link href="https://x.com/Ixirsii" target="_blank">
              <Image
                src="/images/X Logo.svg"
                alt="X"
                width="24"
                height="24"
              />
              X
            </Link>
            <Link href="https://www.youtube.com/@ixirsii" target="_blank">
              <Image
                src="/images/YouTube Logo.svg"
                alt="YouTube"
                width="24"
                height="24"
              />
              YouTube
            </Link>
          </div>
        </div>
      </div>
    </>
  );
}
